//! `tick-fake-server [--port P] [--seed demo|empty|bulk:N] [--token T] [--launch]`
//!
//! Runs the in-memory fake Todoist API. With `--launch` it starts `target/debug/tick` (or
//! `$TICK_BIN`) with `TICK_API_BASE` and `TICK_TOKEN` pointing at the server and exits when
//! the app exits.

use tick_fake_server::{DEFAULT_TOKEN, FakeServer, State, seed};

fn usage() -> ! {
    eprintln!(
        "usage: tick-fake-server [--port P] [--seed demo|empty|bulk:N] [--token T] [--launch]"
    );
    std::process::exit(2);
}

#[tokio::main]
async fn main() {
    let mut port: u16 = 0;
    let mut preset = "demo".to_owned();
    let mut token = DEFAULT_TOKEN.to_owned();
    let mut launch = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" => {
                port = args
                    .next()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or_else(|| usage())
            }
            "--seed" => preset = args.next().unwrap_or_else(|| usage()),
            "--token" => token = args.next().unwrap_or_else(|| usage()),
            "--launch" => launch = true,
            "-h" | "--help" => usage(),
            _ => usage(),
        }
    }
    let mut state = State::new(&token);
    if let Err(e) = seed::apply_preset(&mut state, &preset, 1000) {
        eprintln!("{e}");
        std::process::exit(2);
    }
    let server = match FakeServer::start_with(port, state).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot listen on port {port}: {e}");
            std::process::exit(1);
        }
    };
    let base = server.base_url();
    println!("TICK_API_BASE={base}");
    println!("TICK_TOKEN={token}");
    if launch {
        let bin = std::env::var("TICK_BIN").unwrap_or_else(|_| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            root.join("target/debug/tick")
                .to_string_lossy()
                .into_owned()
        });
        let mut child = match tokio::process::Command::new(&bin)
            .env("TICK_API_BASE", &base)
            .env("TICK_TOKEN", &token)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("cannot start {bin}: {e} (build it with `cargo build -p tick-gtk`)");
                std::process::exit(1);
            }
        };
        tokio::select! {
            status = child.wait() => {
                std::process::exit(status.ok().and_then(|s| s.code()).unwrap_or(0));
            }
            _ = tokio::signal::ctrl_c() => {
                let _ = child.kill().await;
            }
        }
    } else {
        let _ = tokio::signal::ctrl_c().await;
    }
}
