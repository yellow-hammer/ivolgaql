//! TCP-клиент ИволгаQL.

#[cfg(windows)]
const STDIN_EOF_EXIT: &str = "Ctrl+Z и Enter — конец ввода (EOF)";
#[cfg(not(windows))]
const STDIN_EOF_EXIT: &str = "Ctrl+D — конец ввода (EOF)";

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub fn print_usage() {
    eprintln!(
        "Использование: ivolgaql client [--host АДР] [--port ПОРТ] [--no-banner] [--json] [-q|--query ТЕКСТ]\n\
        без -q — интерактив из stdin; с -q — один запрос. --json — ответ одной строкой JSON (для скриптов).\n\
        По умолчанию ответ «помощь» печатается текстом, остальное — разобранный JSON."
    );
}

fn parse_client_args(args: &[String]) -> Result<(String, u16, Option<String>, bool, bool), i32> {
    let mut host = "127.0.0.1".to_string();
    let mut port: u16 = 15432;
    let mut query = None;
    let mut no_banner = false;
    let mut json_output = false;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--host" => {
                i += 1;
                let Some(h) = args.get(i) else {
                    print_usage();
                    return Err(2);
                };
                host = h.clone();
            }
            "--port" => {
                i += 1;
                let Some(p) = args.get(i) else {
                    print_usage();
                    return Err(2);
                };
                port = p.parse().map_err(|_| {
                    eprintln!("неверный --port");
                    2
                })?;
            }
            "-q" | "--query" => {
                i += 1;
                let Some(q) = args.get(i) else {
                    print_usage();
                    return Err(2);
                };
                if query.is_some() {
                    eprintln!("нельзя указать -q|--query более одного раза");
                    return Err(2);
                }
                query = Some(q.clone());
            }
            "--no-banner" => no_banner = true,
            "--json" => json_output = true,
            "-h" | "--help" => {
                print_usage();
                return Err(0);
            }
            other => {
                eprintln!("неизвестный аргумент: {other}");
                print_usage();
                return Err(2);
            }
        }
        i += 1;
    }

    Ok((host, port, query, no_banner, json_output))
}

fn pretty(v: &serde_json::Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

/// Баннер сервера — строки до первой пустой строки (как в [`ivolgaql::BANNER`]).
async fn read_banner<R: AsyncBufReadExt + Unpin>(reader: &mut R) -> Result<String, String> {
    let mut acc = String::new();
    loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| format!("сеть: {e}"))?;
        if n == 0 {
            return if acc.is_empty() {
                Err("сервер закрыл соединение без баннера".into())
            } else {
                Err("обрыв при чтении баннера".into())
            };
        }
        if line == "\n" || line == "\r\n" {
            break;
        }
        acc.push_str(&line);
    }
    Ok(acc)
}

/// В stdout: для «помощь» — читаемый текст; иначе pretty JSON. При `json` — одна строка JSON.
fn print_reply(reply: &serde_json::Value, json: bool) {
    if json {
        println!("{reply}");
        return;
    }
    if reply.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        if let Some(ht) = reply.get("help_text").and_then(|v| v.as_str()) {
            if let Some(b) = reply.get("bird").and_then(|v| v.as_str()) {
                println!("{b}");
            }
            println!();
            println!("{ht}");
            return;
        }
    }
    println!("{}", pretty(reply));
}

pub async fn exchange(
    host: &str,
    port: u16,
    query: &str,
) -> Result<(String, serde_json::Value), String> {
    let stream = TcpStream::connect(format!("{host}:{port}"))
        .await
        .map_err(|e| format!("сеть: {e}"))?;
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    let banner = read_banner(&mut reader).await?;

    let q = query.trim();
    write_half
        .write_all(format!("{q}\n").as_bytes())
        .await
        .map_err(|e| format!("сеть: {e}"))?;
    write_half.flush().await.map_err(|e| format!("сеть: {e}"))?;

    let mut raw = String::new();
    reader
        .read_line(&mut raw)
        .await
        .map_err(|e| format!("сеть: {e}"))?;
    if raw.is_empty() {
        return Err("сервер закрыл соединение без ответа".into());
    }

    let reply: serde_json::Value =
        serde_json::from_str(raw.trim_end()).map_err(|e| format!("ответ не JSON: {e}"))?;

    Ok((banner, reply))
}

pub async fn run(mut args: Vec<String>) -> i32 {
    while args.first().is_some_and(|s| s == "--") {
        args.remove(0);
    }
    let (host, port, query_one, no_banner, json_out) = match parse_client_args(&args) {
        Ok(x) => x,
        Err(0) => return 0,
        Err(code) => return code,
    };

    if let Some(q) = query_one {
        match exchange(&host, port, &q).await {
            Ok((banner, reply)) => {
                if !no_banner && !banner.trim().is_empty() {
                    println!("{}", banner.trim_end());
                }
                print_reply(&reply, json_out);
                if reply.get("ok").and_then(|v| v.as_bool()) == Some(true) {
                    0
                } else {
                    1
                }
            }
            Err(e) => {
                eprintln!("{e}");
                2
            }
        }
    } else {
        let stream = match TcpStream::connect(format!("{host}:{port}")).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("сеть: {e}");
                return 2;
            }
        };

        let (read_half, mut write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);

        let banner = match read_banner(&mut reader).await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{e}");
                return 2;
            }
        };
        if !no_banner && !banner.trim().is_empty() {
            println!("{}", banner.trim_end());
        }

        eprintln!("Интерактив: запрос по строке. Выход — пустая строка или {STDIN_EOF_EXIT}.");

        let stdin = tokio::io::stdin();
        let mut stdin_lines = BufReader::new(stdin).lines();

        let mut exit_code = 0i32;

        loop {
            let line = match stdin_lines.next_line().await {
                Ok(Some(l)) => l,
                Ok(None) => break,
                Err(e) => {
                    eprintln!("{e}");
                    exit_code = 2;
                    break;
                }
            };

            let q = line.trim();
            if q.is_empty() {
                break;
            }

            if write_half
                .write_all(format!("{q}\n").as_bytes())
                .await
                .is_err()
            {
                exit_code = 2;
                break;
            }
            if write_half.flush().await.is_err() {
                exit_code = 2;
                break;
            }

            let mut raw = String::new();
            match reader.read_line(&mut raw).await {
                Ok(0) => {
                    eprintln!("сервер закрыл соединение.");
                    exit_code = 2;
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("сеть: {e}");
                    exit_code = 2;
                    break;
                }
            }

            let reply: serde_json::Value = match serde_json::from_str(raw.trim_end()) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("ответ не JSON: {e}");
                    exit_code = 2;
                    break;
                }
            };

            print_reply(&reply, json_out);
            if reply.get("ok").and_then(|v| v.as_bool()) != Some(true) {
                exit_code = 1;
            }
        }

        let _ = write_half.shutdown().await;
        exit_code
    }
}
