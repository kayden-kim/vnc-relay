//! Reads IP information database files through `MmdbFileGateway`
//! (config-v4 §1.12): the three provider formats and the failure causes.

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use vnc_relay_adapters::ip_db::MmdbFileGateway;
use vnc_relay_app::ip_info::{IpDbError, IpDbFailure, IpDbKey};
use vnc_relay_app::ports::{IpDbFileGateway, IpDbLookupGateway};
use vnc_relay_domain::ip_info::Provider;

#[path = "common/mmdb.rs"]
mod mmdb;

use mmdb::{MmdbWriter, Value, ipinfo, iso_country, map, maxmind_asn, text};

const BUILD_EPOCH: u64 = 1_763_078_400; // 2025-11-14T00:00:00Z

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vnc-relay-ipdb-{}-{name}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .unwrap_or_else(|error| panic!("clear {}: {error}", dir.display()));
    }
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("create {}: {error}", dir.display()));
    dir
}

fn write(dir: &std::path::Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap_or_else(|error| panic!("write {name}: {error}"));
    path
}

fn load(path: &std::path::Path) -> Arc<dyn IpDbLookupGateway> {
    match MmdbFileGateway.read(path).result {
        Ok(db) => db,
        Err(error) => panic!("{} did not load: {error:?}", path.display()),
    }
}

fn load_error(path: &std::path::Path) -> IpDbError {
    match MmdbFileGateway.read(path).result {
        Ok(_) => panic!("{} loaded unexpectedly", path.display()),
        Err(error) => error,
    }
}

fn ip(text: &str) -> IpAddr {
    text.parse()
        .unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn country(db: &dyn IpDbLookupGateway, address: &str) -> Option<String> {
    db.country(ip(address)).map(|code| code.as_str().to_owned())
}

fn asn(db: &dyn IpDbLookupGateway, address: &str) -> Option<u32> {
    db.asn(ip(address)).map(|asn| asn.get())
}

fn as_name(db: &dyn IpDbLookupGateway, address: &str) -> Option<(u32, String)> {
    db.as_name(ip(address)).map(|(asn, name)| (asn.get(), name))
}

/// spec: operations.md:OPS-096 partial
#[test]
fn reads_db_ip_lite_country_and_asn_files() {
    let dir = temp_dir("dbip");
    let country_db = MmdbWriter::new("DBIP-Country-Lite", BUILD_EPOCH)
        // A record without a country comes first; the field is still found.
        .insert(
            "192.0.2.0/25",
            map(&[("continent", map(&[("code", text("AS"))]))]),
        )
        .insert("192.0.2.128/25", iso_country("KR"))
        .insert("2001:db8::/33", iso_country("JP"))
        .insert("198.51.100.0/24", iso_country("kr"));
    let path = write(&dir, "dbip-country-lite.mmdb", &country_db.to_bytes());
    let db = load(&path);
    assert_eq!(db.metadata().database_type, "DBIP-Country-Lite");
    assert_eq!(db.metadata().provider, Provider::Dbip);
    assert_eq!(
        db.metadata().build_time,
        SystemTime::UNIX_EPOCH + Duration::from_secs(BUILD_EPOCH)
    );
    assert!(db.supports(IpDbKey::Country));
    assert!(!db.supports(IpDbKey::Asn));
    assert_eq!(country(db.as_ref(), "192.0.2.200").as_deref(), Some("KR"));
    assert_eq!(country(db.as_ref(), "2001:db8::1").as_deref(), Some("JP"));
    assert_eq!(country(db.as_ref(), "192.0.2.1"), None);
    // Only two upper-case letters are a country.
    assert_eq!(country(db.as_ref(), "198.51.100.7"), None);
    assert_eq!(country(db.as_ref(), "203.0.113.9"), None);

    let asn_db = MmdbWriter::new("DBIP-ASN-Lite (compat=GeoLite2-ASN)", BUILD_EPOCH)
        .insert("192.0.2.0/24", maxmind_asn(64496, "Example Transit"))
        .insert(
            "2001:db8::/32",
            map(&[("autonomous_system_number", Value::U32(64497))]),
        );
    let path = write(&dir, "dbip-asn-lite.mmdb", &asn_db.to_bytes());
    let db = load(&path);
    assert_eq!(db.metadata().provider, Provider::Dbip);
    assert!(!db.supports(IpDbKey::Country));
    assert!(db.supports(IpDbKey::Asn));
    assert_eq!(asn(db.as_ref(), "192.0.2.10"), Some(64496));
    assert_eq!(
        as_name(db.as_ref(), "192.0.2.10"),
        Some((64496, "Example Transit".to_owned()))
    );
    // The organization name is optional.
    assert_eq!(asn(db.as_ref(), "2001:db8:1::5"), Some(64497));
    assert_eq!(as_name(db.as_ref(), "2001:db8:1::5"), None);
    assert_eq!(asn(db.as_ref(), "198.51.100.1"), None);
}

/// spec: operations.md:OPS-096 partial
#[test]
fn reads_one_ipinfo_lite_file_for_country_and_asn() {
    let dir = temp_dir("ipinfo");
    let lite = MmdbWriter::new("ipinfo_lite.mmdb", BUILD_EPOCH)
        .insert("203.0.113.0/24", ipinfo("DE", "AS64500", "Example Hosting"))
        .insert(
            "2001:db8:8000::/33",
            ipinfo("FR", "AS65551", "Example Cloud"),
        )
        // Not `AS` and a decimal number: no ASN.
        .insert("198.51.100.0/24", ipinfo("US", "64501", "Example Lab"));
    let path = write(&dir, "ipinfo_lite.mmdb", &lite.to_bytes());
    let db = load(&path);
    assert_eq!(db.metadata().provider, Provider::Ipinfo);
    assert!(db.supports(IpDbKey::Country));
    assert!(db.supports(IpDbKey::Asn));
    assert_eq!(country(db.as_ref(), "203.0.113.1").as_deref(), Some("DE"));
    assert_eq!(asn(db.as_ref(), "203.0.113.1"), Some(64500));
    assert_eq!(
        as_name(db.as_ref(), "2001:db8:8000::9"),
        Some((65551, "Example Cloud".to_owned()))
    );
    assert_eq!(country(db.as_ref(), "198.51.100.3").as_deref(), Some("US"));
    assert_eq!(asn(db.as_ref(), "198.51.100.3"), None);
    assert_eq!(as_name(db.as_ref(), "198.51.100.3"), None);
}

/// spec: operations.md:OPS-096 partial
#[test]
fn reads_maxmind_geolite_country_city_and_asn_files() {
    let dir = temp_dir("geolite");
    let city = MmdbWriter::new("GeoLite2-City", BUILD_EPOCH).insert(
        "198.51.100.0/24",
        map(&[
            ("city", map(&[("names", map(&[("en", text("Example"))]))])),
            ("country", map(&[("iso_code", text("GB"))])),
            ("location", map(&[("time_zone", text("Europe/London"))])),
        ]),
    );
    let path = write(&dir, "GeoLite2-City.mmdb", &city.to_bytes());
    let db = load(&path);
    assert_eq!(db.metadata().provider, Provider::MaxmindGeolite);
    assert_eq!(
        country(db.as_ref(), "198.51.100.200").as_deref(),
        Some("GB")
    );

    let asn_db = MmdbWriter::new("GeoLite2-ASN", BUILD_EPOCH)
        // A 64-bit unsigned field is read as well.
        .insert(
            "203.0.113.0/24",
            map(&[
                ("autonomous_system_number", Value::U64(65536)),
                ("autonomous_system_organization", text("Example Backbone")),
            ]),
        )
        .insert("2001:db8::/32", maxmind_asn(64511, "Example Edge"));
    let path = write(&dir, "GeoLite2-ASN.mmdb", &asn_db.to_bytes());
    let db = load(&path);
    assert_eq!(db.metadata().provider, Provider::MaxmindGeolite);
    assert!(!db.supports(IpDbKey::Country));
    assert_eq!(
        as_name(db.as_ref(), "203.0.113.77"),
        Some((65536, "Example Backbone".to_owned()))
    );
    assert_eq!(asn(db.as_ref(), "2001:db8::7"), Some(64511));

    let paid =
        MmdbWriter::new("GeoIP2-Country", BUILD_EPOCH).insert("192.0.2.0/24", iso_country("CA"));
    let path = write(&dir, "GeoIP2-Country.mmdb", &paid.to_bytes());
    assert_eq!(load(&path).metadata().provider, Provider::Maxmind);

    let other =
        MmdbWriter::new("Example-Country", BUILD_EPOCH).insert("192.0.2.0/24", iso_country("CA"));
    let path = write(&dir, "other.mmdb", &other.to_bytes());
    let db = load(&path);
    assert_eq!(db.metadata().provider, Provider::Unknown);
    assert_eq!(country(db.as_ref(), "192.0.2.1").as_deref(), Some("CA"));
}

/// spec: operations.md:OPS-096 partial
#[test]
fn a_file_without_the_fields_supports_neither_key() {
    let dir = temp_dir("fields");
    let empty = MmdbWriter::new("DBIP-ASN-Lite", BUILD_EPOCH).insert(
        "192.0.2.0/24",
        map(&[("autonomous_system_organization", text("Name only"))]),
    );
    let path = write(&dir, "fields.mmdb", &empty.to_bytes());
    let db = load(&path);
    assert!(!db.supports(IpDbKey::Country));
    assert!(!db.supports(IpDbKey::Asn));
}

/// spec: operations.md:OPS-096 partial
#[test]
fn each_failure_cause_has_a_fixed_detail_without_the_path() {
    let dir = temp_dir("failures");
    let valid =
        MmdbWriter::new("DBIP-Country-Lite", BUILD_EPOCH).insert("192.0.2.0/24", iso_country("KR"));
    let bytes = valid.to_bytes();
    let cases: Vec<(PathBuf, IpDbFailure)> = vec![
        (dir.join("missing.mmdb"), IpDbFailure::FileUnreadable),
        (dir.clone(), IpDbFailure::FileUnreadable),
        (
            write(&dir, "not-mmdb.mmdb", b"not an MMDB file"),
            IpDbFailure::InvalidFormat,
        ),
        (
            // A copy that has not finished: the metadata is missing.
            write(&dir, "partial.mmdb", &bytes[..bytes.len() / 2]),
            IpDbFailure::InvalidFormat,
        ),
        (
            write(&dir, "separator.mmdb", &valid.to_bytes_with_bad_separator()),
            IpDbFailure::InvalidFormat,
        ),
        (
            write(
                &dir,
                "version.mmdb",
                &MmdbWriter::new("DBIP-Country-Lite", BUILD_EPOCH)
                    .major_version(3)
                    .insert("192.0.2.0/24", iso_country("KR"))
                    .to_bytes(),
            ),
            IpDbFailure::InvalidFormat,
        ),
    ];
    for (path, expected) in cases {
        let error = load_error(&path);
        assert_eq!(error.failure, expected, "{}", path.display());
        let detail = error.detail.unwrap_or_default();
        assert!(!detail.is_empty(), "{}", path.display());
        assert!(
            !detail.contains(&*dir.to_string_lossy()) && !detail.contains("vnc-relay-ipdb"),
            "{detail}"
        );
    }
}

/// spec: operations.md:OPS-096 partial
#[test]
fn a_file_over_256_mib_is_not_read() {
    let dir = temp_dir("large");
    let path = dir.join("large.mmdb");
    let file = std::fs::File::create(&path).expect("create large file");
    // Sparse where the file system allows it; the loader stops at the size.
    file.set_len(256 * 1024 * 1024 + 1)
        .expect("grow large file");
    drop(file);
    let error = load_error(&path);
    assert_eq!(error.failure, IpDbFailure::FileTooLarge);
    std::fs::remove_file(&path).expect("remove large file");

    let exact = dir.join("exact.mmdb");
    let file = std::fs::File::create(&exact).expect("create exact file");
    file.set_len(256 * 1024 * 1024).expect("grow exact file");
    drop(file);
    // At the limit the file is read; zeros are not an MMDB file.
    assert_eq!(load_error(&exact).failure, IpDbFailure::InvalidFormat);
    std::fs::remove_file(&exact).expect("remove exact file");
}
