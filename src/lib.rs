//! g8scope Rust SDK — build the language-neutral error envelope and POST it to the
//! ingest API. `std`-only (raw `TcpStream`, http endpoints). The project is
//! resolved server-side from the ingest key; never sent in the body.

use std::io::Write;
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct Config {
    pub enabled: bool,
    pub key: String,
    pub endpoint: String,
    pub environment: String,
    pub release: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            enabled: true,
            key: String::new(),
            endpoint: "https://ingest.scope.g8suite.com".into(),
            environment: "production".into(),
            release: String::new(),
        }
    }
}

/// Capture an error (type + message) at the given level ("error"/"fatal").
pub fn capture(cfg: &Config, error_type: &str, message: &str, level: &str) {
    if !cfg.enabled || cfg.key.is_empty() {
        return;
    }
    let body = build_envelope(cfg, error_type, message, level);
    let _ = send(cfg, &body); // never break the app
}

pub fn build_envelope(cfg: &Config, error_type: &str, message: &str, level: &str) -> String {
    let release = if cfg.release.is_empty() {
        "null".to_string()
    } else {
        json_str(&cfg.release)
    };
    format!(
        "{{\"schema_version\":\"1.0\",\"event_id\":{},\"occurred_at\":{},\
         \"runtime\":{{\"name\":\"rust\",\"version\":\"{}\"}},\
         \"sdk\":{{\"name\":\"g8scope-rust\",\"version\":\"1.0.0\"}},\
         \"environment\":{},\"release\":{},\"level\":{},\
         \"exception\":{{\"type\":{},\"message\":{},\"frames\":[]}},\
         \"context\":{{\"custom\":{{}}}}}}",
        json_str(&uuid4()),
        json_str(&now_rfc3339()),
        rustc_version(),
        json_str(&cfg.environment),
        release,
        json_str(level),
        json_str(error_type),
        json_str(message),
    )
}

fn send(cfg: &Config, body: &str) -> std::io::Result<()> {
    let raw = cfg
        .endpoint
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let host_port = raw.split('/').next().unwrap_or(raw);
    let host = host_port.split(':').next().unwrap_or(host_port);

    let mut stream = TcpStream::connect(host_port)?;
    let req = format!(
        "POST /v1/ingest HTTP/1.1\r\nHost: {}\r\nX-Ingest-Key: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        host, cfg.key, body.len(), body
    );
    stream.write_all(req.as_bytes())?;
    stream.flush()?;
    // Drain the response so the request completes before the socket closes.
    use std::io::Read;
    let mut resp = String::new();
    let _ = stream.read_to_string(&mut resp);
    Ok(())
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn rustc_version() -> &'static str {
    option_env!("CARGO_PKG_RUST_VERSION").unwrap_or("stable")
}

fn uuid4() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let c = COUNTER.fetch_add(1, Ordering::Relaxed) as u128;
    let mix = nanos
        ^ (c.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        ^ (c << 80);
    let mut b = [0u8; 16];
    for (i, byte) in b.iter_mut().enumerate() {
        *byte = ((mix >> (i * 8)) as u8) ^ ((nanos >> (i * 4)) as u8);
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: Vec<String> = b.iter().map(|x| format!("{:02x}", x)).collect();
    format!(
        "{}{}{}{}-{}{}-{}{}-{}{}-{}{}{}{}{}{}",
        h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], h[8], h[9], h[10], h[11], h[12], h[13], h[14], h[15]
    )
}

fn now_rfc3339() -> String {
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs() as i64;
    let millis = dur.subsec_millis();
    let days = secs.div_euclid(86400);
    let tod = secs.rem_euclid(86400);
    let (y, mo, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y, mo, d, tod / 3600, (tod % 3600) / 60, tod % 60, millis
    )
}

// Howard Hinnant's civil-from-days algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
