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

/// Probe many ports at once; bounded by the longest single probe, not their sum.
pub fn probe_all(ports: &[(u16, Option<String>)]) -> HashMap<u16, bool> {
    let handles: Vec<_> = ports
        .iter()
        .cloned()
        .map(|(port, tech)| std::thread::spawn(move || (port, responds(port, tech.as_deref()))))
        .collect();
    handles.into_iter().filter_map(|h| h.join().ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::responds;
    use std::io::Write;
    use std::net::TcpListener;

    #[test]
    fn answering_server_is_green_and_silent_one_is_not() {
        // Answers every connection with a minimal HTTP response.
        let ok = TcpListener::bind("127.0.0.1:0").unwrap();
        let ok_port = ok.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for c in ok.incoming().flatten() {
                let mut c = c;
                let _ = c.write_all(b"HTTP/1.0 204 No Content\r\n\r\n");
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
