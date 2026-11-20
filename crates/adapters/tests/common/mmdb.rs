//! Writes small MaxMind DB 2.x files for tests (config-v4 §1.12). It covers
//! what the relay reads: an IPv6 search tree with IPv4 under `::/96`, 24-bit
//! records, and data made of maps, arrays, strings and unsigned integers.
//! Format: https://maxmind.github.io/MaxMind-DB/
//!
//! Tests use documentation addresses only (RFC 5737, RFC 3849) and example
//! AS numbers (RFC 5398).

use std::net::IpAddr;

/// One data value.
#[derive(Debug, Clone)]
pub enum Value {
    Str(String),
    U16(u16),
    U32(u32),
    U64(u64),
    Map(Vec<(String, Value)>),
    Array(Vec<Value>),
}

/// A string value.
pub fn text(value: &str) -> Value {
    Value::Str(value.to_owned())
}

/// A map value from key and value pairs.
pub fn map(pairs: &[(&str, Value)]) -> Value {
    Value::Map(
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.clone()))
            .collect(),
    )
}

/// A record of a DB-IP Lite or MaxMind country file.
pub fn iso_country(code: &str) -> Value {
    map(&[(
        "country",
        map(&[
            ("iso_code", text(code)),
            ("names", map(&[("en", text("Example"))])),
        ]),
    )])
}

/// A record of a DB-IP Lite or MaxMind ASN file.
pub fn maxmind_asn(number: u32, organization: &str) -> Value {
    map(&[
        ("autonomous_system_number", Value::U32(number)),
        ("autonomous_system_organization", text(organization)),
    ])
}

/// A record of an IPinfo Lite file. Its `country` is the country name.
pub fn ipinfo(country_code: &str, asn: &str, as_name: &str) -> Value {
    map(&[
        ("asn", text(asn)),
        ("as_name", text(as_name)),
        ("as_domain", text("example.net")),
        ("country_code", text(country_code)),
        ("country", text("Example")),
        ("continent_code", text("AS")),
    ])
}

/// Builds one file. Networks must not overlap.
pub struct MmdbWriter {
    database_type: String,
    build_epoch: u64,
    major_version: u16,
    networks: Vec<(u128, u8, Value)>,
}

#[derive(Clone, Copy)]
enum Record {
    Empty,
    Node(usize),
    Data(usize),
}

impl MmdbWriter {
    pub fn new(database_type: &str, build_epoch: u64) -> Self {
        Self {
            database_type: database_type.to_owned(),
            build_epoch,
            major_version: 2,
            networks: Vec::new(),
        }
    }

    /// Writes another binary format major version into the metadata.
    pub fn major_version(mut self, version: u16) -> Self {
        self.major_version = version;
        self
    }

    /// Adds `network` such as `198.51.100.0/24` or `2001:db8::/32`.
    pub fn insert(mut self, network: &str, value: Value) -> Self {
        let (address, prefix) = network
            .split_once('/')
            .unwrap_or_else(|| panic!("network without prefix: {network}"));
        let address: IpAddr = address
            .parse()
            .unwrap_or_else(|error| panic!("{network}: {error}"));
        let prefix: u8 = prefix
            .parse()
            .unwrap_or_else(|error| panic!("{network}: {error}"));
        let (bits, prefix) = match address {
            IpAddr::V4(v4) => (u128::from(u32::from(v4)), prefix + 96),
            IpAddr::V6(v6) => (u128::from(v6), prefix),
        };
        assert!((1..=128).contains(&prefix), "{network}");
        self.networks.push((bits, prefix, value));
        self
    }

    /// The whole file.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.build().0
    }

    /// The whole file with one byte of the 16-byte separator between the
    /// search tree and the data set to 1. The metadata still parses.
    pub fn to_bytes_with_bad_separator(&self) -> Vec<u8> {
        let (mut bytes, separator) = self.build();
        bytes[separator] = 1;
        bytes
    }

    /// The file and the offset of its data section separator.
    fn build(&self) -> (Vec<u8>, usize) {
        let mut nodes: Vec<[Record; 2]> = vec![[Record::Empty; 2]];
        for (index, (bits, prefix, _)) in self.networks.iter().enumerate() {
            let mut node = 0;
            for depth in 0..*prefix {
                let side = usize::from((bits >> (127 - depth)) & 1 == 1);
                if depth + 1 == *prefix {
                    assert!(
                        matches!(nodes[node][side], Record::Empty),
                        "networks overlap"
                    );
                    nodes[node][side] = Record::Data(index);
                    break;
                }
                node = match nodes[node][side] {
                    Record::Node(next) => next,
                    Record::Empty => {
                        nodes.push([Record::Empty; 2]);
                        let next = nodes.len() - 1;
                        nodes[node][side] = Record::Node(next);
                        next
                    }
                    Record::Data(_) => panic!("networks overlap"),
                };
            }
        }
        let mut data = Vec::new();
        let mut offsets = Vec::new();
        for (_, _, value) in &self.networks {
            offsets.push(data.len());
            encode(&mut data, value);
        }
        let node_count = nodes.len();
        let mut out = Vec::new();
        for node in &nodes {
            for record in node {
                let value = match *record {
                    Record::Empty => node_count,
                    Record::Node(next) => next,
                    Record::Data(index) => node_count + 16 + offsets[index],
                };
                let value = u32::try_from(value).unwrap_or_else(|_| panic!("24-bit record"));
                assert!(value < 1 << 24, "24-bit record");
                out.extend_from_slice(&value.to_be_bytes()[1..]);
            }
        }
        let separator = out.len();
        out.extend_from_slice(&[0; 16]);
        out.extend_from_slice(&data);
        out.extend_from_slice(b"\xAB\xCD\xEFMaxMind.com");
        encode(
            &mut out,
            &map(&[
                (
                    "binary_format_major_version",
                    Value::U16(self.major_version),
                ),
                ("binary_format_minor_version", Value::U16(0)),
                ("build_epoch", Value::U64(self.build_epoch)),
                ("database_type", text(&self.database_type)),
                ("description", map(&[("en", text("Test database"))])),
                ("ip_version", Value::U16(6)),
                ("languages", Value::Array(vec![text("en")])),
                (
                    "node_count",
                    Value::U32(u32::try_from(node_count).unwrap_or_else(|_| panic!("node count"))),
                ),
                ("record_size", Value::U16(24)),
            ]),
        );
        (out, separator)
    }
}

/// Writes the control byte, the extended type byte and the size bytes.
fn control(out: &mut Vec<u8>, kind: u8, size: usize) {
    let (first, extended) = if kind <= 7 {
        (kind << 5, None)
    } else {
        (0, Some(kind - 7))
    };
    let (size_bits, extra): (u8, Vec<u8>) = if size < 29 {
        (u8::try_from(size).unwrap_or_else(size_error), Vec::new())
    } else if size < 285 {
        (29, vec![u8::try_from(size - 29).unwrap_or_else(size_error)])
    } else if size < 65_821 {
        (
            30,
            u16::try_from(size - 285)
                .unwrap_or_else(size_error)
                .to_be_bytes()
                .to_vec(),
        )
    } else {
        let rest = u32::try_from(size - 65_821)
            .unwrap_or_else(size_error)
            .to_be_bytes();
        (31, rest[1..].to_vec())
    };
    out.push(first | size_bits);
    if let Some(extended) = extended {
        out.push(extended);
    }
    out.extend_from_slice(&extra);
}

fn size_error<T>(_: std::num::TryFromIntError) -> T {
    panic!("size does not fit")
}

fn unsigned(out: &mut Vec<u8>, kind: u8, value: u64) {
    let bytes = value.to_be_bytes();
    let start = bytes.iter().position(|byte| *byte != 0).unwrap_or(8);
    control(out, kind, 8 - start);
    out.extend_from_slice(&bytes[start..]);
}

fn encode(out: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Str(text) => {
            control(out, 2, text.len());
            out.extend_from_slice(text.as_bytes());
        }
        Value::U16(number) => unsigned(out, 5, u64::from(*number)),
        Value::U32(number) => unsigned(out, 6, u64::from(*number)),
        Value::U64(number) => unsigned(out, 9, *number),
        Value::Map(pairs) => {
            control(out, 7, pairs.len());
            for (key, value) in pairs {
                encode(out, &Value::Str(key.clone()));
                encode(out, value);
            }
        }
        Value::Array(items) => {
            control(out, 11, items.len());
            for item in items {
                encode(out, item);
            }
        }
    }
}
