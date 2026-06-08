use std::path::Path;

use serde::Deserialize;

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub session: SessionConfig,
    pub rate_limit: RateLimitConfig,
    pub upload: UploadConfig,
    pub admin: AdminConfig,
    pub email: EmailConfig,
    pub seed: SeedConfig,
    pub site: SiteConfig,
    pub mcp: McpConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RedisConfig {
    pub addr: String,
    pub password: String,
    pub db: u8,
    pub pool_size: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SessionConfig {
    pub secret: String,
    pub max_age: u64,
    pub idle_timeout: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    pub login_ip_window_sec: i64,
    pub login_ip_max_attempts: i64,
    pub login_user_fail_threshold: i64,
    pub login_user_cooldown_sec: i64,
    pub registration_ip_window_sec: i64,
    pub registration_ip_max_requests: i64,
    pub registration_email_window_sec: i64,
    pub registration_email_max_requests: i64,
    pub like_ip_window_sec: i64,
    pub like_ip_max_requests: i64,
    pub like_article_window_sec: i64,
    pub like_article_max_actions: i64,
    pub comment_ip_window_sec: i64,
    pub comment_ip_max_requests: i64,
    pub comment_article_window_sec: i64,
    pub comment_article_max_actions: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UploadConfig {
    pub dir: String,
    pub max_size: u64,
    pub allowed_types: Vec<String>,
    pub allow_svg: bool,
    pub reencode: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AdminConfig {
    pub init_username: String,
    pub init_password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
    pub allow_insecure: bool,
    pub verification_ttl_sec: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SeedConfig {
    pub demo_content_enabled: bool,
    pub allow_insecure_admin_password: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SiteConfig {
    pub title: String,
    pub description: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct McpConfig {
    pub enabled: bool,
    pub stdio_enabled: bool,
    pub stdio_write_enabled: bool,
    pub http_enabled: bool,
    pub http_addr: String,
    pub http_path: String,
    pub protocol_versions: Vec<String>,
    pub require_origin_check: bool,
    pub allowed_origins: Vec<String>,
    pub rate_limit: McpRateLimitConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct McpRateLimitConfig {
    pub read_per_minute: i64,
    pub write_per_minute: i64,
    pub publish_per_10min: i64,
    pub upload_per_10min: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            redis: RedisConfig::default(),
            session: SessionConfig::default(),
            rate_limit: RateLimitConfig::default(),
            upload: UploadConfig::default(),
            admin: AdminConfig::default(),
            email: EmailConfig::default(),
            seed: SeedConfig::default(),
            site: SiteConfig::default(),
            mcp: McpConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { port: 3000 }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://localhost:5432/blogweb".into(),
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:6379".into(),
            password: String::new(),
            db: 0,
            pool_size: 10,
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            secret: "change-this-session-secret-to-32-bytes".into(),
            max_age: 86400,
            idle_timeout: 7200,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            login_ip_window_sec: 600,
            login_ip_max_attempts: 20,
            login_user_fail_threshold: 5,
            login_user_cooldown_sec: 900,
            registration_ip_window_sec: 600,
            registration_ip_max_requests: 5,
            registration_email_window_sec: 600,
            registration_email_max_requests: 3,
            like_ip_window_sec: 60,
            like_ip_max_requests: 60,
            like_article_window_sec: 600,
            like_article_max_actions: 20,
            comment_ip_window_sec: 60,
            comment_ip_max_requests: 10,
            comment_article_window_sec: 600,
            comment_article_max_actions: 5,
        }
    }
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            dir: "public/uploads".into(),
            max_size: 5 * 1024 * 1024,
            allowed_types: vec![
                "image/jpeg".into(),
                "image/png".into(),
                "image/gif".into(),
                "image/webp".into(),
            ],
            allow_svg: false,
            reencode: true,
        }
    }
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            init_username: "admin".into(),
            init_password: "change-me-123456".into(),
        }
    }
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            smtp_host: "smtp.163.com".into(),
            smtp_port: 465,
            username: String::new(),
            password: String::new(),
            from: String::new(),
            allow_insecure: false,
            verification_ttl_sec: 600,
        }
    }
}

impl Default for SeedConfig {
    fn default() -> Self {
        Self {
            demo_content_enabled: false,
            allow_insecure_admin_password: false,
        }
    }
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            title: "个人博客".into(),
            description: "一个支持后台管理与 MCP 接入的个人博客系统".into(),
            base_url: "http://localhost:3000".into(),
        }
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            stdio_enabled: true,
            stdio_write_enabled: false,
            http_enabled: false,
            http_addr: "127.0.0.1:3001".into(),
            http_path: "/mcp".into(),
            protocol_versions: vec!["2025-11-25".into()],
            require_origin_check: true,
            allowed_origins: vec![
                "https://chatgpt.com".into(),
                "https://chat.openai.com".into(),
            ],
            rate_limit: McpRateLimitConfig::default(),
        }
    }
}

impl Default for McpRateLimitConfig {
    fn default() -> Self {
        Self {
            read_per_minute: 120,
            write_per_minute: 30,
            publish_per_10min: 10,
            upload_per_10min: 10,
        }
    }
}

pub fn load(path: impl AsRef<Path>) -> Result<Config> {
    let data = std::fs::read_to_string(path)?;
    let config = serde_yaml::from_str::<Config>(&data)?;
    config.validate()?;
    Ok(config)
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.server.port == 0 {
            return Err(AppError::Config(
                "server.port must be greater than 0".into(),
            ));
        }
        if self.database.url.trim().is_empty() {
            return Err(AppError::Config("database.url is required".into()));
        }
        if self.redis.addr.is_empty() {
            return Err(AppError::Config("redis.addr is required".into()));
        }
        if self.session.secret.is_empty() {
            return Err(AppError::Config("session.secret is required".into()));
        }
        if self.session.max_age == 0 {
            return Err(AppError::Config(
                "session.max_age must be greater than 0".into(),
            ));
        }
        if self.session.idle_timeout == 0 {
            return Err(AppError::Config(
                "session.idle_timeout must be greater than 0".into(),
            ));
        }
        if self.rate_limit.login_ip_window_sec <= 0
            || self.rate_limit.login_ip_max_attempts <= 0
            || self.rate_limit.login_user_fail_threshold <= 0
            || self.rate_limit.login_user_cooldown_sec <= 0
            || self.rate_limit.registration_ip_window_sec <= 0
            || self.rate_limit.registration_ip_max_requests <= 0
            || self.rate_limit.registration_email_window_sec <= 0
            || self.rate_limit.registration_email_max_requests <= 0
            || self.rate_limit.like_ip_window_sec <= 0
            || self.rate_limit.like_ip_max_requests <= 0
            || self.rate_limit.like_article_window_sec <= 0
            || self.rate_limit.like_article_max_actions <= 0
            || self.rate_limit.comment_ip_window_sec <= 0
            || self.rate_limit.comment_ip_max_requests <= 0
            || self.rate_limit.comment_article_window_sec <= 0
            || self.rate_limit.comment_article_max_actions <= 0
        {
            return Err(AppError::Config(
                "rate_limit values must be greater than 0".into(),
            ));
        }
        if self.upload.dir.is_empty() {
            return Err(AppError::Config("upload.dir is required".into()));
        }
        if self.upload.max_size == 0 {
            return Err(AppError::Config(
                "upload.max_size must be greater than 0".into(),
            ));
        }
        if self.mcp.http_path.is_empty() || !self.mcp.http_path.starts_with('/') {
            return Err(AppError::Config("mcp.http_path must start with /".into()));
        }
        if self.mcp.protocol_versions.is_empty() {
            return Err(AppError::Config(
                "mcp.protocol_versions must not be empty".into(),
            ));
        }
        if self.mcp.rate_limit.read_per_minute <= 0
            || self.mcp.rate_limit.write_per_minute <= 0
            || self.mcp.rate_limit.publish_per_10min <= 0
            || self.mcp.rate_limit.upload_per_10min <= 0
        {
            return Err(AppError::Config(
                "mcp.rate_limit values must be greater than 0".into(),
            ));
        }
        if self.admin.init_username.is_empty() {
            return Err(AppError::Config("admin.init_username is required".into()));
        }
        if self.email.smtp_port == 0 {
            return Err(AppError::Config(
                "email.smtp_port must be greater than 0".into(),
            ));
        }
        if self.email.verification_ttl_sec == 0 {
            return Err(AppError::Config(
                "email.verification_ttl_sec must be greater than 0".into(),
            ));
        }
        if is_insecure_admin_password(&self.admin.init_password)
            && !self.seed.allow_insecure_admin_password
        {
            return Err(AppError::Config(
                "admin.init_password must be changed before creating a missing admin".into(),
            ));
        }
        Ok(())
    }
}

fn is_insecure_admin_password(value: &str) -> bool {
    matches!(value, "change-me-123456" | "replace-with-secure-password")
}
