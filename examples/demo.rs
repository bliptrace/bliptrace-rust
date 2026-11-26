use bliptrace::{capture, Config};

fn main() {
    let cfg = Config {
        key: std::env::var("BLIPTRACE_KEY").unwrap_or_default(),
        endpoint: std::env::var("BLIPTRACE_ENDPOINT").unwrap_or_else(|_| "http://localhost:8088".into()),
        environment: "demo".into(),
        release: "demo-rust@1.0".into(),
        ..Default::default()
    };

    // Result error
    if let Err(e) = "not-a-number".parse::<i32>() {
        capture(&cfg, "std::num::ParseIntError", &e.to_string(), "error");
        println!("captured: ParseIntError");
    }

    // Caught panic (index out of bounds)
    let res = std::panic::catch_unwind(|| {
        let v = vec![1, 2, 3];
        v[10]
    });
    if res.is_err() {
        capture(&cfg, "panic", "index out of bounds: the len is 3 but the index is 10", "fatal");
        println!("captured: panic");
    }

    // Explicit application error
    capture(&cfg, "AppError", "demo: something went wrong in the worker", "error");
    println!("captured: AppError");

    println!("done");
}
