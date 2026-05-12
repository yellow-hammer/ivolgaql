//! Точка входа: TCP-сервер по умолчанию или подкоманда `client`.

use std::net::SocketAddr;
use std::sync::Arc;

use ivolgaql::{reply_line, run_query, seed_demo, Engine, BANNER};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::Mutex;

fn query_preview(line: &str) -> String {
    let t = line.trim_end_matches(['\r', '\n']);
    const MAX: usize = 72;
    if t.chars().count() <= MAX {
        t.to_string()
    } else {
        t.chars().take(MAX).collect::<String>() + "…"
    }
}

fn ok_label(payload: &Value) -> &'static str {
    match payload.get("ok").and_then(|v| v.as_bool()) {
        Some(true) => "ok",
        Some(false) => "ошибка",
        None => "?",
    }
}

fn normalize_cli_args(mut argv: Vec<String>) -> Vec<String> {
    while argv.first().is_some_and(|s| s == "--") {
        argv.remove(0);
    }
    argv
}

fn cmd_word(s: &str) -> &str {
    s.strip_prefix("--").unwrap_or(s)
}

fn print_main_usage() {
    eprintln!(
        "ИволгаQL — использование бинарника (или через cargo: cargo run -- …):\n\
         \n\
         ivolgaql -h | --help          справка\n\
         ivolgaql                       сервер 127.0.0.1:15432\n\
         ivolgaql ПОРТ                  сервер на порту (только цифры)\n\
         ivolgaql server [ПОРТ]         ivolgaql --server [ПОРТ] — то же\n\
         ivolgaql client …              ivolgaql --client … — см. ivolgaql client --help\n\
         \n\
         Перед аргументами можно написать «--»: ivolgaql -- server 15555"
    );
}

async fn handle_client(
    stream: tokio::net::TcpStream,
    engine: Arc<Mutex<Engine>>,
    peer: SocketAddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    write_half.write_all(BANNER.as_bytes()).await?;

    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }
        let preview = query_preview(&line);
        let payload = {
            let mut eng = engine.lock().await;
            run_query(&mut eng, &line)
        };
        eprintln!("[{peer}] «{preview}» → {}", ok_label(&payload));
        let body = reply_line(&payload)?;
        write_half.write_all(body.as_bytes()).await?;
    }
    Ok(())
}

async fn run_server(host: &str, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let engine = Arc::new(Mutex::new(Engine::new()));
    {
        let mut g = engine.lock().await;
        seed_demo(&mut g);
    }

    let addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&addr).await?;
    eprintln!("ИволгаQL слушает {addr}");

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                eprintln!("остановка по Ctrl+C");
                break;
            }
            res = listener.accept() => {
                let (stream, peer) = res?;
                eprintln!("подключение {peer}");
                let eng = Arc::clone(&engine);
                tokio::spawn(async move {
                    let r = handle_client(stream, eng, peer).await;
                    if let Err(e) = r {
                        eprintln!("[{peer}] ошибка сессии: {e}");
                    }
                    eprintln!("отключился {peer}");
                });
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let argv = normalize_cli_args(std::env::args().skip(1).collect());

    if matches!(
        argv.first().map(|s| s.as_str()),
        Some("-h") | Some("--help")
    ) {
        print_main_usage();
        std::process::exit(0);
    }

    if argv
        .first()
        .is_some_and(|s| cmd_word(s).eq_ignore_ascii_case("client"))
    {
        let code = ivolgaql::client::run(argv.into_iter().skip(1).collect()).await;
        std::process::exit(code);
    }

    let (host, port) = match argv.as_slice() {
        [] => ("127.0.0.1", 15432u16),
        [s] if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) => {
            let p: u16 = match s.parse() {
                Ok(v) => v,
                Err(_) => {
                    print_main_usage();
                    std::process::exit(2);
                }
            };
            ("127.0.0.1", p)
        }
        [cmd, tail @ ..] if cmd_word(cmd).eq_ignore_ascii_case("server") => {
            let p = match tail.first() {
                None => 15432u16,
                Some(s) => match s.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        print_main_usage();
                        std::process::exit(2);
                    }
                },
            };
            ("127.0.0.1", p)
        }
        _ => {
            print_main_usage();
            std::process::exit(2);
        }
    };

    if let Err(e) = run_server(host, port).await {
        eprintln!("сервер: {e}");
        std::process::exit(1);
    }
}
