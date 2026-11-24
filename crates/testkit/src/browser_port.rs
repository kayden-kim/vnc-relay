//! Loopback ports that a headless browser may load. Chromium refuses some
//! ports with `net::ERR_UNSAFE_PORT`. A Windows dynamic port range that starts
//! at 1024 can give such a port to `127.0.0.1:0`. No scenario markers belong in
//! this module.

use std::net::TcpListener;

/// `kRestrictedPorts` of Chromium `net/base/port_util.cc`, without port 0.
const BLOCKED: &[u16] = &[
    1, 7, 9, 11, 13, 15, 17, 19, 20, 21, 22, 23, 25, 37, 42, 43, 53, 69, 77, 79, 87, 95, 101, 102,
    103, 104, 109, 110, 111, 113, 115, 117, 119, 123, 135, 137, 139, 143, 161, 179, 389, 427, 465,
    512, 513, 514, 515, 526, 530, 531, 532, 540, 548, 554, 556, 563, 587, 601, 636, 989, 990, 993,
    995, 1719, 1720, 1723, 2049, 3659, 4045, 5060, 5061, 6000, 6566, 6665, 6666, 6667, 6668, 6669,
    6697, 10080,
];

/// Binds `127.0.0.1` on a port that the operating system chooses and the
/// browser does not block. Rejected listeners stay bound until the return, so
/// each try gets another port.
pub fn bind() -> TcpListener {
    let mut rejected = Vec::new();
    loop {
        let listener = TcpListener::bind("127.0.0.1:0")
            .unwrap_or_else(|error| panic!("bind 127.0.0.1: {error}"));
        let port = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("read the bound address: {error}"))
            .port();
        if !BLOCKED.contains(&port) {
            return listener;
        }
        rejected.push(listener);
    }
}
