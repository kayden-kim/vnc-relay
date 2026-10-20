//! Helpers for TOML text shared by the config and account file readers.

use std::ops::Range;

/// 1-based line and column (in characters) where a TOML parse error starts.
/// Diagnostics give the position only and never quote file content.
pub(crate) fn error_position(text: &str, error: &toml::de::Error) -> Option<(usize, usize)> {
    let span: Range<usize> = error.span()?;
    let before = text.get(..span.start).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        + 1;
    Some((line, column))
}
