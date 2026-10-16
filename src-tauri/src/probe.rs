//! Does a listening port actually answer? TCP connect, plus a tiny HTTP request for
//! web-ish ports: a server that is still booting (or hung) accepts the connection but
//! never writes back.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

const CONNECT: Duration = Duration::from_millis(250);
const RESPONSE: Duration = Duration::from_millis(600);

/// Techs that speak something other than HTTP: a connect is all we can check.
const NOT_HTTP: &[&str] = &["postgres", "mysql", "redis", "docker"];

pub fn responds(port: u16, tech: Option<&str>) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let Ok(mut s) = TcpStream::connect_timeout(&addr, CONNECT) else {
        // Some servers bind IPv6 only.
        return TcpStream::connect_timeout(
            &SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port)),
            CONNECT,
        )
        .is_ok();
    };
    if tech.is_some_and(|t| NOT_HTTP.contains(&t)) {
        return true;
    }
    let _ = s.set_write_timeout(Some(RESPONSE));
    let _ = s.set_read_timeout(Some(RESPONSE));
    if s.write_all(b"HEAD / HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut buf = [0u8; 1];
    matches!(s.read(&mut buf), Ok(n) if n > 0)
}

/// More ports than any machine shows at once; keeps one-thread-per-port bounded.
const MAX_PROBES: usize = 256;

/// Probe many ports at once; bounded by the longest single probe, not their sum.
pub fn probe_all(ports: &[(u16, Option<String>)]) -> HashMap<u16, bool> {
    let mut seen = std::collections::HashSet::new();
    let handles: Vec<_> = ports
        .iter()
        .filter(|(port, _)| seen.insert(*port))
        .take(MAX_PROBES)
        .cloned()
        .map(|(port, tech)| std::thread::spawn(move || (port, responds(port, tech.as_deref()))))
        .collect();
    handles.into_iter().filter_map(|h| h.join().ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::responds;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    #[test]
    fn answering_server_is_green_and_silent_one_is_not() {
        // Answers every connection with a minimal HTTP response, after reading the
        // request like a real server does. That read matters on Windows: closing a
        // socket that still holds unread input sends a reset instead of a clean close,
        // and the peer then loses the response bytes (the probe saw nothing and the
        // test failed at random on the Windows runner).
        let ok = TcpListener::bind("127.0.0.1:0").unwrap();
        let ok_port = ok.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for c in ok.incoming().flatten() {
                let mut c = c;
                let _ = c.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = c.read(&mut [0u8; 512]);
                let _ = c.write_all(b"HTTP/1.0 204 No Content\r\n\r\n");
                let _ = c.flush();
            }
        });
        // Listens but never reads or writes: like a server still booting.
        let silent = TcpListener::bind("127.0.0.1:0").unwrap();
        let silent_port = silent.local_addr().unwrap().port();

        assert!(responds(ok_port, Some("node")));
        assert!(!responds(silent_port, Some("node")));
        // Non-HTTP techs only need the connection to succeed.
        assert!(responds(silent_port, Some("postgres")));
        drop(silent);
    }
}
