use std::time::Duration;

use lettre::{
    message::{Mailbox, Message},
    transport::smtp::{
        authentication::Credentials,
        client::{Tls, TlsParameters},
    },
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};

use crate::{
    config::EmailConfig,
    error::{AppError, Result},
};

pub async fn send_registration_code(
    config: &EmailConfig,
    email: &str,
    code: &str,
    ttl: Duration,
) -> Result<()> {
    let host = config.smtp_host.trim();
    let username = config.username.trim();
    let password = config.password.trim();
    if host.is_empty() || username.is_empty() || password.is_empty() {
        return Err(AppError::HttpJson {
            status: 500,
            code: "email_unavailable".into(),
            message: "邮箱服务尚未配置".into(),
        });
    }
    let from = config.from.trim();
    let from = if from.is_empty() { username } else { from };
    let message = Message::builder()
        .from(parse_mailbox(from)?)
        .to(parse_mailbox(email)?)
        .subject("博客注册验证码")
        .header(lettre::message::header::ContentType::TEXT_PLAIN)
        .body(format!(
            "你的注册验证码是：{code}\n\n验证码 {} 分钟内有效。若非本人操作，请忽略本邮件。",
            ttl.as_secs().max(60) / 60
        ))
        .map_err(|err| AppError::Config(err.to_string()))?;

    let credentials = Credentials::new(username.to_string(), password.to_string());
    let mut builder = if config.allow_insecure {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host).tls(Tls::None)
    } else if config.smtp_port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(host)
            .map_err(|err| AppError::Config(err.to_string()))?
            .tls(Tls::Wrapper(tls_parameters(host)?))
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
            .map_err(|err| AppError::Config(err.to_string()))?
            .tls(Tls::Required(tls_parameters(host)?))
    };
    builder = builder.port(config.smtp_port).credentials(credentials);
    builder
        .build()
        .send(message)
        .await
        .map_err(|err| AppError::Config(err.to_string()))?;
    Ok(())
}

fn parse_mailbox(value: &str) -> Result<Mailbox> {
    value
        .parse::<Mailbox>()
        .map_err(|err| AppError::Config(format!("invalid email address: {err}")))
}

fn tls_parameters(host: &str) -> Result<TlsParameters> {
    TlsParameters::new(host.to_string()).map_err(|err| AppError::Config(err.to_string()))
}
