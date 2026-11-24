//! Reads IP information database files in MaxMind DB (MMDB) 2.x format
//! (config-v4 §1.12, ADR-0012). The whole file is read into memory and
//! checked; lookups never touch the file again.
//! Design: docs/architectures/source-stats.md §4.3.

use std::fs::File;
use std::hash::{DefaultHasher, Hasher};
use std::io::{ErrorKind, Read};
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use maxminddb::{LookupResult, PathElement, Reader, WithinOptions};
use vnc_relay_application::ip_info::{
    Fingerprint, IpDbError, IpDbKey, IpDbMetadata, IpDbRead, IpDbReadError,
};
use vnc_relay_application::ports::{IpDbFileGateway, IpDbLookupGateway};
use vnc_relay_domain::ip_info::{Asn, CountryCode};

/// Largest file read: 256 MiB (config-v4 §1.12).
const SIZE_LIMIT: u64 = 256 * 1024 * 1024;

/// Records looked at to decide the fields (config-v4 §1.12 검사).
const SAMPLE_RECORDS: usize = 4096;

const COUNTRY_CODE: &[PathElement<'static>] = &[PathElement::Key("country_code")];
const COUNTRY_ISO_CODE: &[PathElement<'static>] =
    &[PathElement::Key("country"), PathElement::Key("iso_code")];
const IPINFO_ASN: &[PathElement<'static>] = &[PathElement::Key("asn")];
const IPINFO_AS_NAME: &[PathElement<'static>] = &[PathElement::Key("as_name")];
const MAXMIND_ASN: &[PathElement<'static>] = &[PathElement::Key("autonomous_system_number")];
const MAXMIND_AS_NAME: &[PathElement<'static>] =
    &[PathElement::Key("autonomous_system_organization")];

/// The country field a file uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CountryField {
    /// IPinfo Lite `country_code`.
    CountryCode,
    /// DB-IP and MaxMind `country.iso_code`.
    IsoCode,
}

/// The ASN fields a file uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AsnField {
    /// IPinfo Lite `asn` ("AS" and digits) and `as_name`.
    Ipinfo,
    /// DB-IP and MaxMind `autonomous_system_number` and
    /// `autonomous_system_organization`.
    Maxmind,
}

/// Reads files from the local file system.
pub struct MmdbFileGateway;

impl IpDbFileGateway for MmdbFileGateway {
    fn read(&self, path: &Path) -> IpDbRead {
        match read_bytes(path) {
            Ok((bytes, hash)) => IpDbRead {
                fingerprint: Fingerprint::Content(hash),
                result: open(bytes).map(|db| Arc::new(db) as Arc<dyn IpDbLookupGateway>),
            },
            Err((fingerprint, error)) => IpDbRead {
                fingerprint,
                result: Err(error),
            },
        }
    }
}

/// The file's bytes and a hash of them, or why it cannot be used.
fn read_bytes(path: &Path) -> Result<(Vec<u8>, u64), (Fingerprint, IpDbReadError)> {
    let unreadable = |error: std::io::Error| {
        (
            Fingerprint::Unreadable,
            failure(IpDbError::FileUnreadable, io_cause(&error)),
        )
    };
    let too_large = |size: u64| {
        (
            Fingerprint::TooLarge(size),
            failure(
                IpDbError::FileTooLarge,
                "파일이 256 MiB를 넘는다.".to_owned(),
            ),
        )
    };
    let file = File::open(path).map_err(unreadable)?;
    let metadata = file.metadata().map_err(unreadable)?;
    if metadata.is_dir() {
        return Err((
            Fingerprint::Unreadable,
            failure(
                IpDbError::FileUnreadable,
                "파일이 아니라 디렉터리다.".to_owned(),
            ),
        ));
    }
    if metadata.len() > SIZE_LIMIT {
        return Err(too_large(metadata.len()));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    // The file can grow after the size check; never read past the limit.
    file.take(SIZE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    let read = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if read > SIZE_LIMIT {
        return Err(too_large(read));
    }
    let mut hasher = DefaultHasher::new();
    hasher.write(&bytes);
    Ok((bytes, hasher.finish()))
}

/// The cause of an I/O failure without the path (config-v4 §1.12 rule 6).
fn io_cause(error: &std::io::Error) -> String {
    match error.kind() {
        ErrorKind::NotFound => "파일을 찾을 수 없다.".to_owned(),
        ErrorKind::PermissionDenied => "읽을 권한이 없다.".to_owned(),
        ErrorKind::IsADirectory => "파일이 아니라 디렉터리다.".to_owned(),
        _ => format!("{error}."),
    }
}

fn failure(failure: IpDbError, detail: String) -> IpDbReadError {
    IpDbReadError {
        failure,
        detail: Some(detail),
    }
}

/// Checks the whole file and picks the fields. The detail sentences are
/// fixed: messages from the reader can quote keys from the file.
fn open(bytes: Vec<u8>) -> Result<MmdbLookupGateway, IpDbReadError> {
    let invalid = |detail: &str| failure(IpDbError::InvalidFormat, detail.to_owned());
    // `from_source` also rejects a binary format major version other than 2.
    let reader = Reader::from_source(bytes)
        .map_err(|_| invalid("MMDB 2.x 메타데이터나 검색 트리를 읽지 못했다."))?;
    reader
        .verify()
        .map_err(|_| invalid("MMDB 검색 트리나 데이터 영역 검사에 실패했다."))?;
    let (country, asn) = pick_fields(&reader).map_err(|()| invalid("레코드를 읽지 못했다."))?;
    let source = reader.metadata();
    let build_time = SystemTime::UNIX_EPOCH + Duration::from_secs(source.build_epoch);
    let metadata = IpDbMetadata::new(&source.database_type, build_time);
    Ok(MmdbLookupGateway {
        reader,
        metadata,
        country,
        asn,
    })
}

/// The MaxMind-style AS number of a record. MaxMind and DB-IP write
/// `uint32`; the reader decodes a fixed width only, so the value goes
/// through a JSON number to accept any unsigned width.
fn maxmind_asn_number(record: &LookupResult<'_, Vec<u8>>) -> Option<u64> {
    record
        .decode_path::<serde_json::Value>(MAXMIND_ASN)
        .ok()
        .flatten()?
        .as_u64()
}

/// The first listed field that any of the first 4096 records has wins.
fn pick_fields(reader: &Reader<Vec<u8>>) -> Result<(Option<CountryField>, Option<AsnField>), ()> {
    let records = reader
        .networks(WithinOptions::default().skip_empty_values())
        .map_err(|_| ())?;
    let mut found = [false; 4];
    for record in records.take(SAMPLE_RECORDS) {
        let record = record.map_err(|_| ())?;
        let has_text = |path: &[PathElement<'_>]| matches!(record.decode_path::<&str>(path), Ok(Some(text)) if !text.is_empty());
        found[0] |= has_text(COUNTRY_CODE);
        found[1] |= has_text(COUNTRY_ISO_CODE);
        found[2] |= has_text(IPINFO_ASN);
        found[3] |= maxmind_asn_number(&record).is_some();
        if found[0] && found[2] {
            break;
        }
    }
    let country = if found[0] {
        Some(CountryField::CountryCode)
    } else if found[1] {
        Some(CountryField::IsoCode)
    } else {
        None
    };
    let asn = if found[2] {
        Some(AsnField::Ipinfo)
    } else if found[3] {
        Some(AsnField::Maxmind)
    } else {
        None
    };
    Ok((country, asn))
}

/// A checked file held in memory.
pub struct MmdbLookupGateway {
    reader: Reader<Vec<u8>>,
    metadata: IpDbMetadata,
    country: Option<CountryField>,
    asn: Option<AsnField>,
}

impl MmdbLookupGateway {
    fn text(&self, ip: IpAddr, path: &[PathElement<'_>]) -> Option<&str> {
        self.reader
            .lookup(ip)
            .ok()?
            .decode_path::<&str>(path)
            .ok()
            .flatten()
    }
}

impl IpDbLookupGateway for MmdbLookupGateway {
    fn metadata(&self) -> &IpDbMetadata {
        &self.metadata
    }

    fn supports(&self, key: IpDbKey) -> bool {
        match key {
            IpDbKey::Country => self.country.is_some(),
            IpDbKey::Asn => self.asn.is_some(),
        }
    }

    fn country(&self, ip: IpAddr) -> Option<CountryCode> {
        let path = match self.country? {
            CountryField::CountryCode => COUNTRY_CODE,
            CountryField::IsoCode => COUNTRY_ISO_CODE,
        };
        self.text(ip, path).and_then(CountryCode::parse)
    }

    fn asn(&self, ip: IpAddr) -> Option<Asn> {
        match self.asn? {
            AsnField::Ipinfo => self.text(ip, IPINFO_ASN).and_then(Asn::parse_prefixed),
            AsnField::Maxmind => maxmind_asn_number(&self.reader.lookup(ip).ok()?)
                .and_then(|number| u32::try_from(number).ok())
                .and_then(Asn::new),
        }
    }

    fn as_name(&self, ip: IpAddr) -> Option<(Asn, String)> {
        let asn = self.asn(ip)?;
        let path = match self.asn? {
            AsnField::Ipinfo => IPINFO_AS_NAME,
            AsnField::Maxmind => MAXMIND_AS_NAME,
        };
        let name = self.text(ip, path).filter(|name| !name.is_empty())?;
        Some((asn, name.to_owned()))
    }
}
