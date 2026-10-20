//! Reads one TOML table key by key, records which keys were read, and turns
//! type and range problems into diagnostics.

use std::ops::RangeInclusive;

use toml::{Table, Value};
use vnc_relay_app::settings::{ConfigDiagnostic, ConfigError};

/// All diagnostics found so far, in the order they were found.
#[derive(Debug, Default)]
pub(super) struct Diagnostics(Vec<ConfigDiagnostic>);

impl Diagnostics {
    pub(super) fn push(
        &mut self,
        error: ConfigError,
        key: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.0.push(ConfigDiagnostic {
            error,
            key: key.into(),
            message: message.into(),
        });
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn into_vec(self) -> Vec<ConfigDiagnostic> {
        self.0
    }
}

/// State of one key after reading. `Invalid` means a diagnostic was pushed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Field<T> {
    Absent,
    Invalid,
    Value(T),
}

impl<T> Field<T> {
    /// The value, or `default` when the key is absent. `None` when invalid.
    pub(super) fn or(self, default: T) -> Option<T> {
        match self {
            Self::Absent => Some(default),
            Self::Invalid => None,
            Self::Value(value) => Some(value),
        }
    }

    pub(super) fn value(self) -> Option<T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Absent | Self::Invalid => None,
        }
    }

    pub(super) fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    pub(super) fn map<U>(self, f: impl FnOnce(T) -> Option<U>) -> Field<U> {
        match self {
            Self::Absent => Field::Absent,
            Self::Invalid => Field::Invalid,
            Self::Value(value) => f(value).map_or(Field::Invalid, Field::Value),
        }
    }
}

/// One TOML table and the keys read from it so far. A missing table reads
/// as a table without keys.
pub(super) struct Section<'a> {
    path: String,
    table: Option<&'a Table>,
    read: Vec<&'static str>,
}

impl<'a> Section<'a> {
    pub(super) fn root(table: &'a Table) -> Self {
        Self {
            path: String::new(),
            table: Some(table),
            read: Vec::new(),
        }
    }

    /// Full dotted path of a key in this table.
    pub(super) fn key_path(&self, key: &str) -> String {
        if self.path.is_empty() {
            key.to_owned()
        } else {
            format!("{}.{key}", self.path)
        }
    }

    fn get(&mut self, key: &'static str) -> Option<&'a Value> {
        self.read.push(key);
        self.table.and_then(|table| table.get(key))
    }

    pub(super) fn section(&mut self, key: &'static str, d: &mut Diagnostics) -> Section<'a> {
        let path = self.key_path(key);
        let table = match self.get(key) {
            None => None,
            Some(Value::Table(table)) => Some(table),
            Some(other) => {
                type_mismatch(d, &path, "테이블", other);
                None
            }
        };
        Section {
            path,
            table,
            read: Vec::new(),
        }
    }

    pub(super) fn bool(&mut self, key: &'static str, d: &mut Diagnostics) -> Field<bool> {
        match self.get(key) {
            None => Field::Absent,
            Some(Value::Boolean(value)) => Field::Value(*value),
            Some(other) => {
                type_mismatch(d, &self.key_path(key), "불리언", other);
                Field::Invalid
            }
        }
    }

    pub(super) fn integer(
        &mut self,
        key: &'static str,
        range: RangeInclusive<i64>,
        d: &mut Diagnostics,
    ) -> Field<i64> {
        match self.get(key) {
            None => Field::Absent,
            Some(Value::Integer(value)) if range.contains(value) => Field::Value(*value),
            Some(Value::Integer(_)) => {
                out_of_range(d, self.key_path(key), *range.start(), *range.end());
                Field::Invalid
            }
            Some(other) => {
                type_mismatch(d, &self.key_path(key), "정수", other);
                Field::Invalid
            }
        }
    }

    pub(super) fn string(&mut self, key: &'static str, d: &mut Diagnostics) -> Field<&'a str> {
        match self.get(key) {
            None => Field::Absent,
            Some(Value::String(value)) => Field::Value(value),
            Some(other) => {
                type_mismatch(d, &self.key_path(key), "문자열", other);
                Field::Invalid
            }
        }
    }

    /// Array of strings. A non-string element is reported at `key[i]`.
    pub(super) fn string_array(
        &mut self,
        key: &'static str,
        d: &mut Diagnostics,
    ) -> Field<Vec<&'a str>> {
        let path = self.key_path(key);
        match self.get(key) {
            None => Field::Absent,
            Some(Value::Array(values)) => {
                let mut strings = Vec::with_capacity(values.len());
                let mut valid = true;
                for (index, value) in values.iter().enumerate() {
                    match value {
                        Value::String(text) => strings.push(text.as_str()),
                        other => {
                            type_mismatch(d, &format!("{path}[{index}]"), "문자열", other);
                            valid = false;
                        }
                    }
                }
                if valid {
                    Field::Value(strings)
                } else {
                    Field::Invalid
                }
            }
            Some(other) => {
                type_mismatch(d, &path, "문자열 배열", other);
                Field::Invalid
            }
        }
    }

    /// Reports every key of this table that was not read.
    pub(super) fn finish(self, d: &mut Diagnostics) {
        let Some(table) = self.table else { return };
        for (key, value) in table {
            if !self.read.contains(&key.as_str()) {
                let what = if value.is_table() { "테이블" } else { "키" };
                d.push(
                    ConfigError::UnknownKey,
                    self.key_path(key),
                    format!("알 수 없는 {what}다. 설정 계약에 없는 이름이다."),
                );
            }
        }
    }
}

pub(super) fn out_of_range(d: &mut Diagnostics, key: String, min: i64, max: i64) {
    d.push(
        ConfigError::ValueOutOfRange,
        key,
        format!("값이 유효 범위 {min}~{max} 밖이다."),
    );
}

fn type_mismatch(d: &mut Diagnostics, key: &str, expected: &str, actual: &Value) {
    d.push(
        ConfigError::TypeMismatch,
        key,
        format!(
            "자료형이 다르다. 기대: {expected}, 실제: {}.",
            type_name(actual)
        ),
    );
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "문자열",
        Value::Integer(_) => "정수",
        Value::Float(_) => "실수",
        Value::Boolean(_) => "불리언",
        Value::Datetime(_) => "날짜·시간",
        Value::Array(_) => "배열",
        Value::Table(_) => "테이블",
    }
}
