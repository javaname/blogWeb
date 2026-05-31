#![allow(dead_code)]

use blogweb::{app, config::Config};
use sqlx::{Pool, Sqlite};
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};

pub const ADMIN_PASSWORD: &str = "admin-password";
pub const ADMIN_PASSWORD_HASH: &str =
    "$2b$04$d43519q9RUOpZqsj0sfN4ej74bM4Z3PVCG5IGgNtPshwioMLY0LC2";

#[derive(Clone)]
pub struct FakeRedis {
    addr: String,
    store: Arc<Mutex<HashMap<String, String>>>,
}

impl FakeRedis {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake redis");
        let addr = listener.local_addr().expect("fake redis addr").to_string();
        let store = Arc::new(Mutex::new(HashMap::new()));
        let server_store = Arc::clone(&store);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let connection_store = Arc::clone(&server_store);
                thread::spawn(move || handle_connection(stream, connection_store));
            }
        });
        Self { addr, store }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.store
            .lock()
            .expect("fake redis lock")
            .get(key)
            .cloned()
    }

    pub fn addr(&self) -> &str {
        &self.addr
    }
}

pub fn router_with_redis(pool: Pool<Sqlite>, redis: &FakeRedis) -> axum::Router {
    let mut config = Config::default();
    config.redis.addr = redis.addr.clone();
    app::router_with_pool_and_config(
        pool,
        std::path::PathBuf::from("public/assets"),
        std::path::PathBuf::from("public/uploads"),
        config,
    )
}

#[derive(Clone)]
pub struct FakeSmtp {
    addr: String,
    messages: Arc<Mutex<Vec<String>>>,
}

impl FakeSmtp {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake smtp");
        let addr = listener.local_addr().expect("fake smtp addr").to_string();
        let messages = Arc::new(Mutex::new(Vec::new()));
        let server_messages = Arc::clone(&messages);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let connection_messages = Arc::clone(&server_messages);
                thread::spawn(move || handle_smtp_connection(stream, connection_messages));
            }
        });
        Self { addr, messages }
    }

    pub fn host(&self) -> String {
        self.addr
            .split(':')
            .next()
            .unwrap_or("127.0.0.1")
            .to_string()
    }

    pub fn port(&self) -> u16 {
        self.addr
            .rsplit_once(':')
            .and_then(|(_, port)| port.parse::<u16>().ok())
            .expect("fake smtp port")
    }

    pub fn messages(&self) -> Vec<String> {
        self.messages.lock().expect("fake smtp lock").clone()
    }
}

fn handle_connection(stream: TcpStream, store: Arc<Mutex<HashMap<String, String>>>) {
    let mut reader = BufReader::new(stream);
    loop {
        let command = match read_command(&mut reader) {
            Ok(Some(command)) => command,
            Ok(None) | Err(_) => return,
        };
        if command.is_empty() {
            continue;
        }
        let name = command[0].to_ascii_uppercase();
        let response = match name.as_str() {
            "PING" => "+PONG\r\n".to_string(),
            "AUTH" | "SELECT" | "SET" => {
                if name == "SET" && command.len() >= 3 {
                    store
                        .lock()
                        .expect("fake redis lock")
                        .insert(command[1].clone(), command[2].clone());
                }
                "+OK\r\n".to_string()
            }
            "GET" => {
                let value = command
                    .get(1)
                    .and_then(|key| store.lock().expect("fake redis lock").get(key).cloned());
                match value {
                    Some(value) => format!("${}\r\n{}\r\n", value.len(), value),
                    None => "$-1\r\n".to_string(),
                }
            }
            "INCR" => {
                let Some(key) = command.get(1) else {
                    let _ = reader
                        .get_mut()
                        .write_all(b"-ERR wrong number of arguments\r\n");
                    continue;
                };
                let mut data = store.lock().expect("fake redis lock");
                let next = data
                    .get(key)
                    .and_then(|value| value.parse::<i64>().ok())
                    .unwrap_or_default()
                    + 1;
                data.insert(key.clone(), next.to_string());
                format!(":{next}\r\n")
            }
            "EXPIRE" => ":1\r\n".to_string(),
            "DEL" => {
                let mut removed = 0;
                if let Ok(mut data) = store.lock() {
                    for key in command.iter().skip(1) {
                        if data.remove(key).is_some() {
                            removed += 1;
                        }
                    }
                }
                format!(":{removed}\r\n")
            }
            _ => "-ERR unknown command\r\n".to_string(),
        };
        if reader.get_mut().write_all(response.as_bytes()).is_err() {
            return;
        }
    }
}

fn read_command(reader: &mut BufReader<TcpStream>) -> std::io::Result<Option<Vec<String>>> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None);
    }
    if !line.starts_with('*') {
        return Ok(None);
    }
    let count = line[1..].trim().parse::<usize>().unwrap_or_default();
    let mut command = Vec::with_capacity(count);
    for _ in 0..count {
        line.clear();
        reader.read_line(&mut line)?;
        let len = line[1..].trim().parse::<usize>().unwrap_or_default();
        let mut data = vec![0; len + 2];
        reader.read_exact(&mut data)?;
        command.push(String::from_utf8_lossy(&data[..len]).to_string());
    }
    Ok(Some(command))
}

fn handle_smtp_connection(stream: TcpStream, messages: Arc<Mutex<Vec<String>>>) {
    let mut reader = BufReader::new(stream);
    if reader
        .get_mut()
        .write_all(b"220 fake-smtp ESMTP\r\n")
        .is_err()
    {
        return;
    }
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or_default() == 0 {
            return;
        }
        let command = line.trim_end_matches(['\r', '\n']);
        let upper = command.to_ascii_uppercase();
        let response = if upper.starts_with("EHLO") || upper.starts_with("HELO") {
            "250-fake-smtp\r\n250-AUTH PLAIN LOGIN\r\n250 OK\r\n"
        } else if upper.starts_with("AUTH") {
            "235 Authentication successful\r\n"
        } else if upper.starts_with("MAIL FROM") || upper.starts_with("RCPT TO") {
            "250 OK\r\n"
        } else if upper == "DATA" {
            if reader
                .get_mut()
                .write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n")
                .is_err()
            {
                return;
            }
            let mut data = String::new();
            loop {
                line.clear();
                if reader.read_line(&mut line).unwrap_or_default() == 0 {
                    return;
                }
                if line.trim_end_matches(['\r', '\n']) == "." {
                    break;
                }
                data.push_str(&line);
            }
            messages.lock().expect("fake smtp lock").push(data);
            "250 Queued\r\n"
        } else if upper == "QUIT" {
            let _ = reader.get_mut().write_all(b"221 Bye\r\n");
            return;
        } else {
            "250 OK\r\n"
        };
        if reader.get_mut().write_all(response.as_bytes()).is_err() {
            return;
        }
    }
}
