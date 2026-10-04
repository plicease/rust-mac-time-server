//! RFC 868 time server (UDP).
//!
//! The server listens for UDP datagrams; whenever one arrives, it replies to
//! the sender with the current time as a 32-bit big-endian count of seconds
//! since 1900-01-01 00:00:00 UTC. The contents of the request are ignored.

use anyhow::{Context, Result, bail};
use std::net::UdpSocket;
use std::time::{SystemTime, UNIX_EPOCH};

/// Address to listen on when none is given. Port 37 is the RFC 868 port.
const DEFAULT_LISTEN: &str = "0.0.0.0:37";

/// Seconds between 1900-01-01 and 1970-01-01 (the Unix epoch).
const SECONDS_1900_TO_1970: u64 = 2_208_988_800;

/// Converts a time to RFC 868 seconds since 1900.
///
/// The value is 32 bits wide and wraps in February 2036, as the protocol
/// specifies no other behaviour.
fn rfc868_time(now: SystemTime) -> Result<u32> {
    let unix = now
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_secs();
    Ok((unix + SECONDS_1900_TO_1970) as u32)
}

/// Answers requests on `socket` forever.
fn serve(socket: &UdpSocket) -> Result<()> {
    // Any request content is accepted, so a small buffer is enough; longer
    // datagrams are truncated, which is harmless.
    let mut buf = [0u8; 512];
    loop {
        let (_, peer) = match socket.recv_from(&mut buf) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("receive failed: {e}");
                continue;
            }
        };
        let reply = match rfc868_time(SystemTime::now()) {
            Ok(t) => t.to_be_bytes(),
            Err(e) => {
                eprintln!("{e:#}");
                continue;
            }
        };
        if let Err(e) = socket.send_to(&reply, peer) {
            eprintln!("send to {peer} failed: {e}");
        }
    }
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let listen = match (args.next(), args.next()) {
        (None, _) => DEFAULT_LISTEN.to_string(),
        (Some(a), None) if a == "-h" || a == "--help" => {
            println!("usage: mac-time-server [ADDRESS:PORT]  (default {DEFAULT_LISTEN})");
            return Ok(());
        }
        (Some(a), None) => a,
        (Some(_), Some(_)) => bail!("usage: mac-time-server [ADDRESS:PORT]"),
    };

    let socket = UdpSocket::bind(&listen).with_context(|| format!("cannot bind to {listen}"))?;
    eprintln!("listening on udp {}", socket.local_addr()?);
    serve(&socket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn unix_epoch_is_rfc868_offset() {
        assert_eq!(rfc868_time(UNIX_EPOCH).unwrap(), 2_208_988_800);
    }

    #[test]
    fn rfc868_examples() {
        // Examples from RFC 868, paired with the Unix time of the same instant.
        let cases = [
            (189_302_400, 2_398_291_200u32), // 1976-01-01 00:00:00 UTC
            (315_532_800, 2_524_521_600u32), // 1980-01-01 00:00:00 UTC
            (420_595_200, 2_629_584_000u32), // 1983-05-01 00:00:00 UTC
        ];
        for (unix, expected) in cases {
            let t = UNIX_EPOCH + Duration::from_secs(unix);
            assert_eq!(rfc868_time(t).unwrap(), expected);
        }
    }

    #[test]
    fn wraps_in_2036() {
        // 2036-02-07 06:28:16 UTC is 2^32 seconds after 1900.
        let t = UNIX_EPOCH + Duration::from_secs((1u64 << 32) - SECONDS_1900_TO_1970);
        assert_eq!(rfc868_time(t).unwrap(), 0);
    }

    #[test]
    fn serves_time_over_udp() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = server.local_addr().unwrap();
        std::thread::spawn(move || serve(&server));

        let client = UdpSocket::bind("127.0.0.1:0").unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();

        let before = rfc868_time(SystemTime::now()).unwrap();
        client.send_to(&[], addr).unwrap();
        let mut buf = [0u8; 16];
        let (n, from) = client.recv_from(&mut buf).unwrap();
        let after = rfc868_time(SystemTime::now()).unwrap();

        assert_eq!(n, 4);
        assert_eq!(from, addr);
        let got = u32::from_be_bytes(buf[..4].try_into().unwrap());
        assert!(
            before <= got && got <= after,
            "{before} <= {got} <= {after}"
        );
    }
}
