//! Byte-preserving config edits and same-directory atomic replacement.
//! Design: docs/architectures/config-edit.md; config-v4 §1.10.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use aws_lc_rs::digest::{SHA256, digest};
use toml_edit::{Document, Item, Table, Value};
use vnc_relay_app::management::config::{
    ConfigChange, ConfigDocument, ConfigFileStore, ConfigReplaceError, ConfigValue,
};
use vnc_relay_app::reload::ConfigCandidate;
use vnc_relay_app::settings::{ConfigDiagnostic, ConfigError};

/// Raw-file SHA-256 revisions and atomic, conflict-checked replacement.
#[derive(Debug, Default, Clone, Copy)]
pub struct FsConfigFileStore;

impl ConfigFileStore for FsConfigFileStore {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn revision(&self, bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let hash = digest(&SHA256, bytes);
        let mut revision = String::with_capacity(71);
        revision.push_str("sha256:");
        for byte in hash.as_ref() {
            revision.push(HEX[usize::from(byte >> 4)] as char);
            revision.push(HEX[usize::from(byte & 15)] as char);
        }
        revision
    }

    fn replace(
        &self,
        path: &Path,
        expected_revision: &str,
        next: &[u8],
    ) -> Result<(), ConfigReplaceError> {
        replace_file(path, expected_revision, next, |_, _| Ok(()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteStage {
    Created,
    Synced,
}

/// The hook is private: tests can stop at real filesystem boundaries without
/// widening the production port or pretending the final check is a lock.
fn replace_file(
    path: &Path,
    expected_revision: &str,
    next: &[u8],
    mut hook: impl FnMut(WriteStage, &Path) -> io::Result<()>,
) -> Result<(), ConfigReplaceError> {
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    let fail = |_| ConfigReplaceError::WriteFailed;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path.file_name().ok_or(ConfigReplaceError::WriteFailed)?;
    let mut temp_name = std::ffi::OsString::from(".");
    temp_name.push(name);
    temp_name.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
    let temp_path = parent.join(temp_name);
    let mut temp = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(fail)?;
    let mut cleanup = TempFile {
        path: temp_path,
        active: true,
    };
    hook(WriteStage::Created, &cleanup.path).map_err(fail)?;
    temp.write_all(next).map_err(fail)?;
    // Keep the original file's permission bits rather than widening access.
    if let Ok(metadata) = fs::metadata(path) {
        temp.set_permissions(metadata.permissions()).map_err(fail)?;
    }
    temp.flush().map_err(fail)?;
    temp.sync_all().map_err(fail)?;
    drop(temp);
    hook(WriteStage::Synced, &cleanup.path).map_err(fail)?;
    // Q178: this is deliberately the final I/O before rename. External writes
    // between this comparison and rename remain outside the guarantee.
    let current = fs::read(path).map_err(|_| ConfigReplaceError::Conflict)?;
    if FsConfigFileStore.revision(&current) != expected_revision {
        return Err(ConfigReplaceError::Conflict);
    }
    fs::rename(&cleanup.path, path).map_err(fail)?;
    cleanup.active = false;
    // Do not report a post-rename directory-sync failure as an unchanged file.
    Ok(())
}

struct TempFile {
    path: PathBuf,
    active: bool,
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if self.active {
            #[cfg(windows)]
            #[expect(
                clippy::permissions_set_readonly_false,
                reason = "clears only the Windows read-only attribute copied from the original so the temp file can be deleted"
            )]
            if let Ok(metadata) = fs::metadata(&self.path) {
                let mut permissions = metadata.permissions();
                permissions.set_readonly(false);
                let _ = fs::set_permissions(&self.path, permissions);
            }
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Uses the config loader for validation and TOML source spans for editing.
/// Untouched bytes are never sent through a document serializer.
#[derive(Debug, Default, Clone, Copy)]
pub struct TomlConfigDocument;

impl ConfigDocument for TomlConfigDocument {
    fn parse(
        &self,
        bytes: &[u8],
        base_dir: &Path,
    ) -> Result<ConfigCandidate, Vec<ConfigDiagnostic>> {
        crate::config_file::parse_candidate(bytes, base_dir)
    }

    fn edit(
        &self,
        bytes: &[u8],
        changes: &[ConfigChange],
    ) -> Result<Vec<u8>, Vec<ConfigDiagnostic>> {
        let text =
            std::str::from_utf8(bytes).map_err(|_| diagnostic(ConfigError::FileUnreadable, ""))?;
        let (bom, source) = match text.strip_prefix('\u{feff}') {
            Some(source) => ("\u{feff}", source),
            None => ("", text),
        };
        if changes.is_empty() {
            Document::parse(source).map_err(|_| diagnostic(ConfigError::SyntaxError, ""))?;
            return Ok(bytes.to_vec());
        }
        let newline = first_newline(source);
        let mut source = source.to_owned();
        for change in changes {
            let doc = Document::parse(source.as_str())
                .map_err(|_| diagnostic(ConfigError::SyntaxError, ""))?;
            let parts: Vec<_> = change.key.split('.').collect();
            if parts.iter().any(|part| part.is_empty()) || matches!(change.value, ConfigValue::Null)
            {
                return Err(diagnostic(ConfigError::TypeMismatch, &change.key));
            }
            let edit = edit_change(&doc, &source, &parts, &change.value, newline)
                .ok_or_else(|| diagnostic(ConfigError::TypeMismatch, &change.key))?;
            if let Some((range, replacement)) = edit {
                source.replace_range(range, &replacement);
            }
        }
        let mut result = Vec::with_capacity(bom.len() + source.len());
        result.extend_from_slice(bom.as_bytes());
        result.extend_from_slice(source.as_bytes());
        Ok(result)
    }
}

fn diagnostic(error: ConfigError, key: &str) -> Vec<ConfigDiagnostic> {
    vec![ConfigDiagnostic {
        error,
        key: key.to_owned(),
        message: match error {
            ConfigError::FileUnreadable => "설정 파일이 UTF-8 텍스트가 아니다.",
            ConfigError::SyntaxError => "TOML 구문 오류 또는 중복 키다.",
            _ => "설정 값의 자료형이 올바르지 않다.",
        }
        .to_owned(),
    }]
}

fn first_newline(text: &str) -> &'static str {
    match text.find('\n') {
        Some(index) if index > 0 && text.as_bytes()[index - 1] == b'\r' => "\r\n",
        _ => "\n",
    }
}

fn same_value(old: &Value, next: &ConfigValue) -> bool {
    match next {
        ConfigValue::Bool(next) => old.as_bool() == Some(*next),
        ConfigValue::Integer(next) => old.as_integer() == Some(*next),
        ConfigValue::String(next) => old.as_str() == Some(next.as_str()),
        ConfigValue::Strings(next) => old.as_array().is_some_and(|array| {
            array.len() == next.len()
                && array
                    .iter()
                    .zip(next)
                    .all(|(old, next)| old.as_str() == Some(next.as_str()))
        }),
        ConfigValue::Null => false,
    }
}

fn value_text(value: &ConfigValue) -> Option<String> {
    Some(match value {
        ConfigValue::Bool(value) => value.to_string(),
        ConfigValue::Integer(value) => value.to_string(),
        ConfigValue::String(value) => Value::from(value.as_str()).to_string(),
        ConfigValue::Strings(values) => {
            let mut array = toml_edit::Array::new();
            for value in values {
                array.push(value.as_str());
            }
            Value::Array(array).to_string()
        }
        ConfigValue::Null => return None,
    })
}

type TextEdit = (Range<usize>, String);

fn edit_change(
    doc: &Document<&str>,
    source: &str,
    parts: &[&str],
    next: &ConfigValue,
    newline: &str,
) -> Option<Option<TextEdit>> {
    let mut current = doc.as_item();
    let mut scope = doc.as_table();
    let mut relative_start = 0;
    let mut inline_scope = None;
    for (index, part) in parts.iter().enumerate() {
        let table = current.as_table_like()?;
        let Some(child) = table.get(part) else {
            let text = value_text(next)?;
            // Inline tables may contain dotted keys, including absent parents.
            if let Some((inline, start)) = inline_scope {
                let inline: &toml_edit::InlineTable = inline;
                let span = inline.span()?;
                let at = span.end - 1;
                let separator = if inline.is_empty() || inline.trailing_comma() {
                    " "
                } else {
                    ", "
                };
                return Some(Some((
                    at..at,
                    format!("{separator}{} = {text}", parts[start..].join(".")),
                )));
            }
            if index + 1 < parts.len()
                || current
                    .as_table()
                    .is_some_and(|table| table.is_implicit() && !table.is_dotted())
            {
                // Append the full parent path, including missing listener/role
                // or ticket tables. Never insert beneath the last explicit sibling.
                let at = source.len();
                let separator = if source.is_empty() || source.ends_with('\n') {
                    ""
                } else {
                    newline
                };
                return Some(Some((
                    at..at,
                    format!(
                        "{separator}[{}]{newline}{} = {text}{newline}",
                        parts[..parts.len() - 1].join("."),
                        parts.last()?
                    ),
                )));
            }
            let at = scope_end(scope, source);
            let separator = if at == 0 || source[..at].ends_with('\n') {
                ""
            } else {
                newline
            };
            return Some(Some((
                at..at,
                format!(
                    "{separator}{} = {text}{newline}",
                    parts[relative_start..].join(".")
                ),
            )));
        };
        if index + 1 == parts.len() {
            let old = child.as_value()?;
            if same_value(old, next) {
                return Some(None);
            }
            let range = old.span()?;
            let replacement = match (old.as_array(), next) {
                (Some(array), ConfigValue::Strings(values)) => {
                    edit_array(array, source, values, newline)?
                }
                _ => value_text(next)?,
            };
            return Some(Some((range, replacement)));
        }
        current = child;
        if let Some(inline) = child.as_inline_table()
            && !inline.is_dotted()
        {
            inline_scope = Some((inline, index + 1));
        }
        if let Some(table) = child.as_table()
            && !table.is_dotted()
            && !table.is_implicit()
        {
            scope = table;
            relative_start = index + 1;
        }
    }
    None
}

fn scope_end(table: &Table, source: &str) -> usize {
    let mut end = table.span().map_or(0, |span| {
        if span.is_empty() {
            0
        } else {
            line_end(source, span.start)
        }
    });
    for (_, item) in table.iter() {
        match item {
            Item::Value(value) => {
                if let Some(span) = value.span() {
                    end = end.max(line_end(source, span.end));
                }
            }
            Item::Table(child) if child.is_dotted() => {
                end = end.max(scope_end(child, source));
            }
            _ => {}
        }
    }
    end
}

fn line_end(text: &str, from: usize) -> usize {
    text[from..]
        .find('\n')
        .map_or(text.len(), |offset| from + offset + 1)
}

// Preserve the gaps between array tokens verbatim. These gaps carry comments,
// indentation and mixed line endings even when an element is removed.
fn edit_array(
    array: &toml_edit::Array,
    source: &str,
    values: &[String],
    newline: &str,
) -> Option<String> {
    let span = array.span()?;
    let mut result = String::from("[");
    let mut cursor = span.start + 1;
    let mut indent = " ";
    let multiline = source[span.clone()].contains('\n');
    for (index, old) in array.iter().enumerate() {
        let old_span = old.span()?;
        let gap = &source[cursor..old_span.start];
        if index >= values.len() {
            result.push_str(&without_commas(gap));
        } else {
            result.push_str(gap);
            if old.as_str() == Some(values[index].as_str()) {
                result.push_str(&source[old_span.clone()]);
            } else {
                result.push_str(&Value::from(values[index].as_str()).to_string());
            }
        }
        if let Some(start) = gap.rfind('\n') {
            let candidate = &gap[start + 1..];
            if candidate.bytes().all(|byte| byte == b' ' || byte == b'\t') {
                indent = candidate;
            }
        }
        cursor = old_span.end;
    }
    let tail = &source[cursor..span.end - 1];
    if values.len() > array.len() {
        if !array.is_empty() && !array.trailing_comma() {
            result.push(',');
        }
        // Keep a last element's trailing comment beside that element. New
        // entries go after the original tail instead of stealing its decor.
        result.push_str(tail);
        for (index, value) in values.iter().enumerate().skip(array.len()) {
            if index > array.len() {
                result.push(',');
            }
            if multiline {
                result.push_str(newline);
                result.push_str(indent);
            } else if index > 0 {
                result.push(' ');
            }
            result.push_str(&Value::from(value.as_str()).to_string());
        }
        if array.trailing_comma() {
            result.push(',');
        }
        if multiline {
            result.push_str(newline);
        }
    } else if values.len() < array.len() {
        result.push_str(&without_commas(tail));
    } else {
        result.push_str(tail);
    }
    result.push(']');
    Some(result)
}

fn without_commas(gap: &str) -> String {
    let mut result = String::with_capacity(gap.len());
    let mut comment = false;
    for ch in gap.chars() {
        match ch {
            '#' => comment = true,
            '\n' => comment = false,
            ',' if !comment => continue,
            _ => {}
        }
        result.push(ch);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// spec: management.md:MGT-053 partial
    /// spec: operations.md:OPS-060 partial
    #[test]
    fn missing_ticket_tables_append_and_later_keys_preserve_existing_bytes() {
        for newline in ["\n", "\r\n"] {
            for source in [
                "# untouched\n[relay.ws.viewer]\nenabled = true # keep\n",
                "# untouched\n[relay.ticket.server]\nenabled = false # keep\n",
                "# untouched\n[relay.ticket]\n# keep parent\n[mode1]\nenabled = false",
            ] {
                let source = source.replace('\n', newline);
                let changes = [
                    ConfigChange {
                        key: "relay.ticket.viewer.enabled".to_owned(),
                        value: ConfigValue::Bool(false),
                    },
                    ConfigChange {
                        key: "relay.ticket.viewer.issuer".to_owned(),
                        value: ConfigValue::String(" issuer.example ".to_owned()),
                    },
                    ConfigChange {
                        key: "relay.ticket.viewer.relay_host".to_owned(),
                        value: ConfigValue::String("relay.example".to_owned()),
                    },
                    ConfigChange {
                        key: "relay.ticket.viewer.max_lifetime_secs".to_owned(),
                        value: ConfigValue::Integer(90),
                    },
                ];
                let result = TomlConfigDocument
                    .edit(source.as_bytes(), &changes)
                    .unwrap_or_else(|error| panic!("edit ticket: {error:?}"));
                let separator = if source.ends_with('\n') { "" } else { newline };
                assert_eq!(
                    String::from_utf8(result.clone()).unwrap_or_default(),
                    format!(
                        "{source}{separator}[relay.ticket.viewer]{newline}enabled = false{newline}issuer = \" issuer.example \"{newline}relay_host = \"relay.example\"{newline}max_lifetime_secs = 90{newline}"
                    )
                );
                assert_eq!(
                    TomlConfigDocument.edit(&result, &changes),
                    Ok(result),
                    "repeating the edit must preserve all bytes"
                );
            }
        }
    }

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let dir = std::env::temp_dir().join(format!(
                "vnc-relay-config-replace-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&dir).unwrap_or_else(|error| panic!("create fixture: {error}"));
            fs::write(dir.join("relay.toml"), b"original")
                .unwrap_or_else(|error| panic!("write fixture: {error}"));
            Self(dir)
        }

        fn path(&self) -> PathBuf {
            self.0.join("relay.toml")
        }

        fn assert_no_temp(&self, expected: usize) {
            assert_eq!(
                fs::read_dir(&self.0)
                    .unwrap_or_else(|error| panic!("read directory: {error}"))
                    .count(),
                expected
            );
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap_or_else(|error| panic!("remove fixture: {error}"));
        }
    }

    /// spec: management.md:MGT-033 partial
    #[test]
    fn external_save_after_temp_sync_is_rechecked_and_temp_is_removed() {
        let fixture = Fixture::new();
        let path = fixture.path();
        let revision = FsConfigFileStore.revision(b"original");
        let mut stages = Vec::new();
        let mut external_modified = None;
        let result = replace_file(&path, &revision, b"candidate", |stage, temp| {
            stages.push(stage);
            if stage == WriteStage::Synced {
                assert_eq!(fs::read(temp)?, b"candidate");
                fs::write(&path, b"external comment-only save")?;
                external_modified = fs::metadata(&path)?.modified().ok();
            }
            Ok(())
        });
        assert_eq!(stages, [WriteStage::Created, WriteStage::Synced]);
        assert_eq!(result, Err(ConfigReplaceError::Conflict));
        assert_eq!(
            fs::read(&path).unwrap_or_else(|error| panic!("read: {error}")),
            b"external comment-only save"
        );
        assert_eq!(
            fs::metadata(&path)
                .unwrap_or_else(|error| panic!("metadata: {error}"))
                .modified()
                .ok(),
            external_modified
        );
        fixture.assert_no_temp(1);
    }

    /// spec: management.md:MGT-033 partial
    #[test]
    fn unreadable_final_revision_check_is_conflict_and_cleans_temp() {
        let fixture = Fixture::new();
        let path = fixture.path();
        let revision = FsConfigFileStore.revision(b"original");
        let result = replace_file(&path, &revision, b"candidate", |stage, _| {
            if stage == WriteStage::Synced {
                fs::remove_file(&path)?;
            }
            Ok(())
        });
        assert_eq!(result, Err(ConfigReplaceError::Conflict));
        assert!(!path.exists());
        fixture.assert_no_temp(0);
    }

    /// spec: management.md:MGT-038 partial
    /// spec: operations.md:OPS-061 partial
    #[test]
    fn injected_write_stage_failures_leave_original_metadata_and_clean_temp() {
        for failing_stage in [WriteStage::Created, WriteStage::Synced] {
            let fixture = Fixture::new();
            let path = fixture.path();
            let before = fs::metadata(&path).unwrap_or_else(|error| panic!("metadata: {error}"));
            let result = replace_file(
                &path,
                &FsConfigFileStore.revision(b"original"),
                b"candidate",
                |stage, _| {
                    if stage == failing_stage {
                        Err(io::Error::new(
                            io::ErrorKind::PermissionDenied,
                            "injected write failure",
                        ))
                    } else {
                        Ok(())
                    }
                },
            );
            assert_eq!(result, Err(ConfigReplaceError::WriteFailed));
            assert_eq!(
                fs::read(&path).unwrap_or_else(|error| panic!("read: {error}")),
                b"original"
            );
            let after = fs::metadata(&path).unwrap_or_else(|error| panic!("metadata: {error}"));
            assert_eq!(before.modified().ok(), after.modified().ok());
            assert_eq!(before.permissions(), after.permissions());
            fixture.assert_no_temp(1);
        }
    }

    /// spec: management.md:MGT-038 partial
    /// spec: operations.md:OPS-061 partial
    #[test]
    fn real_rename_failure_does_not_delete_or_rewrite_original() {
        let fixture = Fixture::new();
        let path = fixture.path();
        let before = fs::metadata(&path).unwrap_or_else(|error| panic!("metadata: {error}"));
        let result = replace_file(
            &path,
            &FsConfigFileStore.revision(b"original"),
            b"candidate",
            |stage, temp| {
                if stage == WriteStage::Synced {
                    fs::remove_file(temp)?;
                }
                Ok(())
            },
        );
        assert_eq!(result, Err(ConfigReplaceError::WriteFailed));
        assert_eq!(
            fs::read(&path).unwrap_or_else(|error| panic!("read: {error}")),
            b"original"
        );
        assert_eq!(
            before.modified().ok(),
            fs::metadata(&path)
                .unwrap_or_else(|error| panic!("metadata: {error}"))
                .modified()
                .ok()
        );
        fixture.assert_no_temp(1);
    }
}
