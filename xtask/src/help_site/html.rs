//! Start tags of the HTML that mdBook and this repository write.
//!
//! This is not a general HTML parser. It skips comments, `<!…>` and `<?…>`
//! declarations, end tags, and the text of `script` and `style` elements. It
//! returns each start tag with its lower-case name and attributes in source
//! order. The help site checks need nothing more.

/// One start tag.
pub struct Tag {
    /// Lower-case element name.
    pub name: String,
    /// Lower-case attribute names and decoded values. An attribute without a value has `""`.
    pub attributes: Vec<(String, String)>,
    /// Byte offset of `<` in the source.
    pub offset: usize,
}

impl Tag {
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// Start tags of `source` in order.
pub fn start_tags(source: &str) -> Vec<Tag> {
    let mut tags = Vec::new();
    let mut index = 0;
    while let Some(found) = source[index..].find('<') {
        let start = index + found;
        let rest = &source[start..];
        if rest.starts_with("<!--") {
            index = rest.find("-->").map_or(source.len(), |end| start + end + 3);
            continue;
        }
        if rest.starts_with("<!") || rest.starts_with("<?") || rest.starts_with("</") {
            index = rest.find('>').map_or(source.len(), |end| start + end + 1);
            continue;
        }
        let Some((tag, end)) = read_tag(source, start) else {
            // A `<` in text, such as `a < b`.
            index = start + 1;
            continue;
        };
        index = end;
        if tag.name == "script" || tag.name == "style" {
            // Raw text ends only at the matching end tag.
            let close = format!("</{}", tag.name);
            index = find_ignoring_case(&source[end..], &close).map_or(source.len(), |at| end + at);
        }
        tags.push(tag);
    }
    tags
}

/// Reads the tag that starts at `start` (a `<`). Returns the tag and the byte
/// offset after its `>`, or `None` when no element name follows the `<`.
fn read_tag(source: &str, start: usize) -> Option<(Tag, usize)> {
    let bytes = source.as_bytes();
    let mut index = start + 1;
    if !bytes.get(index)?.is_ascii_alphabetic() {
        return None;
    }
    let name_start = index;
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
    {
        index += 1;
    }
    let mut tag = Tag {
        name: source[name_start..index].to_ascii_lowercase(),
        attributes: Vec::new(),
        offset: start,
    };
    loop {
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        match bytes.get(index) {
            None => return Some((tag, source.len())),
            Some(b'>') => return Some((tag, index + 1)),
            Some(b'/') => {
                index += 1;
                continue;
            }
            Some(_) => {}
        }
        let key_start = index;
        while bytes
            .get(index)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b'=' | b'>' | b'/'))
        {
            index += 1;
        }
        let key = source[key_start..index].to_ascii_lowercase();
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        let mut value = String::new();
        if bytes.get(index) == Some(&b'=') {
            index += 1;
            while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
                index += 1;
            }
            let raw = match bytes.get(index) {
                Some(quote @ (b'"' | b'\'')) => {
                    let value_start = index + 1;
                    let end = source[value_start..]
                        .find(*quote as char)
                        .map_or(source.len(), |at| value_start + at);
                    index = (end + 1).min(source.len());
                    &source[value_start..end]
                }
                _ => {
                    let value_start = index;
                    while bytes
                        .get(index)
                        .is_some_and(|byte| !byte.is_ascii_whitespace() && *byte != b'>')
                    {
                        index += 1;
                    }
                    &source[value_start..index]
                }
            };
            value = decode_references(raw);
        }
        tag.attributes.push((key, value));
    }
}

/// Decodes the character references this site's HTML uses in attribute values:
/// `&amp;`, `&quot;`, `&#39;`, `&lt;`, `&gt;`, and numeric references. Others stay as they are.
fn decode_references(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';') else {
            break;
        };
        let name = &rest[1..end];
        let decoded = match name {
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "lt" => Some('<'),
            "gt" => Some('>'),
            _ => name
                .strip_prefix("#x")
                .or_else(|| name.strip_prefix("#X"))
                .map(|hex| u32::from_str_radix(hex, 16))
                .or_else(|| name.strip_prefix('#').map(str::parse::<u32>))
                .and_then(Result::ok)
                .and_then(char::from_u32),
        };
        match decoded {
            Some(character) => {
                out.push(character);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Byte offset of the first ASCII case-insensitive match of `needle` in `haystack`.
fn find_ignoring_case(haystack: &str, needle: &str) -> Option<usize> {
    let needle = needle.as_bytes();
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(source: &str) -> Vec<String> {
        start_tags(source)
            .iter()
            .map(|tag| {
                let attributes: Vec<String> = tag
                    .attributes
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect();
                format!("{}[{}]", tag.name, attributes.join(" "))
            })
            .collect()
    }

    #[test]
    fn markup_inside_comments_scripts_and_styles_is_not_read_as_tags() {
        let source = concat!(
            "<!DOCTYPE html><!-- <a href=\"comment.html\"> -->",
            "<script>const html = '<a href=\"script.html\">'; if (a < b) {}</SCRIPT>",
            "<style>a::after { content: \"<img src=x>\"; }</style>",
            "<p>1 < 2</p><a href=\"page.html\">x</a>"
        );
        assert_eq!(
            summary(source),
            ["script[]", "style[]", "p[]", "a[href=page.html]"]
        );
    }

    #[test]
    fn attribute_forms_and_references_are_read() {
        let source = "<IMG SRC='images/a.png' alt=\"A &amp; B &#39;c&#x27;\" hidden data-x=1/>";
        assert_eq!(
            summary(source),
            ["img[src=images/a.png alt=A & B 'c' hidden= data-x=1/]"]
        );
    }
}
