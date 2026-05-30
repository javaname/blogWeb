use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    config::{Config, RedisConfig, SessionConfig},
    error::{AppError, Result},
};

#[derive(Clone)]
pub struct RedisSessionStore {
    redis: RedisClient,
    config: SessionConfig,
}

#[derive(Clone)]
struct RedisClient {
    config: RedisConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUser {
    #[serde(rename = "user_id")]
    pub(crate) id: i64,
    pub(crate) username: String,
    pub(crate) role: String,
    pub(crate) csrf_token: String,
    pub(crate) created_at: u64,
    pub(crate) last_seen: u64,
}

enum RedisValue {
    Simple(String),
    Integer(i64),
    Bulk(Option<String>),
}

impl RedisSessionStore {
    pub fn new(config: &Config) -> Self {
        Self {
            redis: RedisClient {
                config: config.redis.clone(),
            },
            config: config.session.clone(),
        }
    }

    pub async fn create(
        &self,
        id: i64,
        username: String,
        role: String,
    ) -> Result<(String, SessionUser)> {
        let session_id = secure_token();
        let csrf_token = secure_token();
        let now = unix_now();
        let session = SessionUser {
            id,
            username,
            role,
            csrf_token,
            created_at: now,
            last_seen: now,
        };
        self.save(&session_id, &session).await?;
        Ok((session_id, session))
    }

    pub async fn get(&self, session_id: &str) -> Result<Option<SessionUser>> {
        if session_id.is_empty() {
            return Ok(None);
        }
        let Some(raw) = self.redis.get(&session_key(session_id)).await? else {
            return Ok(None);
        };
        let mut session: SessionUser = serde_json::from_str(&raw)?;
        let now = unix_now();
        if now.saturating_sub(session.created_at) > self.config.max_age
            || now.saturating_sub(session.last_seen) > self.config.idle_timeout
        {
            self.destroy(session_id).await?;
            return Ok(None);
        }
        session.last_seen = now;
        self.save(session_id, &session).await?;
        Ok(Some(session))
    }

    pub async fn destroy(&self, session_id: &str) -> Result<()> {
        if session_id.is_empty() {
            return Ok(());
        }
        self.redis
            .del(&[session_key(session_id), csrf_key(session_id)])
            .await?;
        Ok(())
    }

    pub async fn allow_rate_limit(
        &self,
        key: &str,
        max_attempts: i64,
        window_sec: i64,
    ) -> Result<bool> {
        if max_attempts <= 0 {
            return Ok(true);
        }
        let count = self
            .redis
            .incr_with_expiry(key, window_sec.max(1) as u64)
            .await?;
        Ok(count <= max_attempts)
    }

    async fn save(&self, session_id: &str, session: &SessionUser) -> Result<()> {
        let data = serde_json::to_string(session)?;
        let ttl = self.config.max_age;
        self.redis
            .set_ex(&session_key(session_id), &data, ttl)
            .await?;
        self.redis
            .set_ex(&csrf_key(session_id), &session.csrf_token, ttl)
            .await?;
        Ok(())
    }
}

impl RedisClient {
    async fn set_ex(&self, key: &str, value: &str, ttl: u64) -> Result<()> {
        match self
            .command(&["SET", key, value, "EX", &ttl.to_string()])
            .await?
        {
            RedisValue::Simple(value) if value.eq_ignore_ascii_case("OK") => Ok(()),
            _ => Err(AppError::Config(
                "redis SET returned unexpected response".into(),
            )),
        }
    }

    async fn get(&self, key: &str) -> Result<Option<String>> {
        match self.command(&["GET", key]).await? {
            RedisValue::Bulk(value) => Ok(value),
            _ => Err(AppError::Config(
                "redis GET returned unexpected response".into(),
            )),
        }
    }

    async fn del(&self, keys: &[String]) -> Result<i64> {
        let mut args = Vec::with_capacity(keys.len() + 1);
        args.push("DEL".to_string());
        args.extend(keys.iter().cloned());
        let borrowed = args.iter().map(String::as_str).collect::<Vec<_>>();
        match self.command(&borrowed).await? {
            RedisValue::Integer(value) => Ok(value),
            _ => Err(AppError::Config(
                "redis DEL returned unexpected response".into(),
            )),
        }
    }

    async fn incr_with_expiry(&self, key: &str, ttl: u64) -> Result<i64> {
        let count = match self.command(&["INCR", key]).await? {
            RedisValue::Integer(value) => value,
            _ => {
                return Err(AppError::Config(
                    "redis INCR returned unexpected response".into(),
                ))
            }
        };
        if count == 1 {
            match self.command(&["EXPIRE", key, &ttl.to_string()]).await? {
                RedisValue::Integer(_) => {}
                _ => {
                    return Err(AppError::Config(
                        "redis EXPIRE returned unexpected response".into(),
                    ))
                }
            }
        }
        Ok(count)
    }

    async fn command(&self, args: &[&str]) -> Result<RedisValue> {
        let mut stream = TcpStream::connect(&self.config.addr).await?;
        if !self.config.password.is_empty() {
            write_command(&mut stream, &["AUTH", &self.config.password]).await?;
            expect_ok(read_response(&mut stream).await?)?;
        }
        if self.config.db != 0 {
            write_command(&mut stream, &["SELECT", &self.config.db.to_string()]).await?;
            expect_ok(read_response(&mut stream).await?)?;
        }
        write_command(&mut stream, args).await?;
        read_response(&mut stream).await
    }
}

fn expect_ok(value: RedisValue) -> Result<()> {
    match value {
        RedisValue::Simple(value) if value.eq_ignore_ascii_case("OK") => Ok(()),
        _ => Err(AppError::Config("redis auth/select failed".into())),
    }
}

async fn write_command(stream: &mut TcpStream, args: &[&str]) -> Result<()> {
    let mut buffer = format!("*{}\r\n", args.len()).into_bytes();
    for arg in args {
        buffer.extend_from_slice(format!("${}\r\n", arg.len()).as_bytes());
        buffer.extend_from_slice(arg.as_bytes());
        buffer.extend_from_slice(b"\r\n");
    }
    stream.write_all(&buffer).await?;
    Ok(())
}

async fn read_response(stream: &mut TcpStream) -> Result<RedisValue> {
    let prefix = read_byte(stream).await?;
    match prefix {
        b'+' => Ok(RedisValue::Simple(read_line(stream).await?)),
        b'-' => Err(AppError::Config(format!(
            "redis error: {}",
            read_line(stream).await?
        ))),
        b':' => {
            let value = read_line(stream)
                .await?
                .parse::<i64>()
                .map_err(|err| AppError::Config(format!("invalid redis integer: {err}")))?;
            Ok(RedisValue::Integer(value))
        }
        b'$' => {
            let len = read_line(stream)
                .await?
                .parse::<isize>()
                .map_err(|err| AppError::Config(format!("invalid redis bulk length: {err}")))?;
            if len < 0 {
                return Ok(RedisValue::Bulk(None));
            }
            let mut data = vec![0; len as usize + 2];
            stream.read_exact(&mut data).await?;
            Ok(RedisValue::Bulk(Some(
                String::from_utf8_lossy(&data[..len as usize]).to_string(),
            )))
        }
        _ => Err(AppError::Config("invalid redis response".into())),
    }
}

async fn read_byte(stream: &mut TcpStream) -> Result<u8> {
    let mut byte = [0];
    stream.read_exact(&mut byte).await?;
    Ok(byte[0])
}

async fn read_line(stream: &mut TcpStream) -> Result<String> {
    let mut bytes = Vec::new();
    loop {
        let byte = read_byte(stream).await?;
        bytes.push(byte);
        if bytes.ends_with(b"\r\n") {
            bytes.truncate(bytes.len() - 2);
            return Ok(String::from_utf8_lossy(&bytes).to_string());
        }
    }
}

fn session_key(session_id: &str) -> String {
    format!("session:{session_id}")
}

fn csrf_key(session_id: &str) -> String {
    format!("csrf:{session_id}")
}

fn secure_token() -> String {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
