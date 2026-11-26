# Bliptrace (Rust)

Rust SDK for [Bliptrace](https://bliptrace.com) — build the language-neutral error envelope and
ship it to the ingest API. **`std`-only** (raw `TcpStream`, http endpoints), zero dependencies.

```rust
use bliptrace::{capture, Config};

let cfg = Config { key: std::env::var("BLIPTRACE_KEY").unwrap_or_default(),
    endpoint: "http://localhost:8088".into(), environment: "production".into(),
    release: "my-app@1.0".into(), ..Default::default() };

capture(&cfg, "std::io::Error", "disk full", "error");
```

The project is resolved server-side from the ingest key; never sent in the body. `capture` never
panics into the host app. For TLS endpoints, swap the transport for a `reqwest`/`ureq` build.

_Developers Hub Sdn Bhd · bliptrace-rust_
