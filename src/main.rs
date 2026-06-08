use std::ffi::OsString;

use blogweb::{app, config, db, error::Result};
use clap::{Args, Parser, Subcommand};
use tokio::net::TcpListener;

#[derive(Debug, Parser)]
#[command(name = "blogweb")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    ServeWeb(ConfigArg),
    ServeMcp(ServeMcpArgs),
    Mcp(McpCommand),
    Db(DbCommand),
}

#[derive(Debug, Args)]
struct DbCommand {
    #[command(subcommand)]
    action: DbAction,
}

#[derive(Debug, Args)]
struct ServeMcpArgs {
    #[arg(
        long = "config",
        short = 'c',
        alias = "config",
        default_value = "config.yaml"
    )]
    config: String,
    #[arg(long = "transport", alias = "transport", default_value = "stdio")]
    transport: String,
}

#[derive(Debug, Args)]
struct McpCommand {
    #[command(subcommand)]
    action: McpAction,
}

#[derive(Debug, Subcommand)]
enum McpAction {
    IssueToken(IssueTokenArgs),
    RevokeToken(RevokeTokenArgs),
}

#[derive(Debug, Args)]
struct IssueTokenArgs {
    #[arg(
        long = "config",
        short = 'c',
        alias = "config",
        default_value = "config.yaml"
    )]
    config: String,
    #[arg(long = "name", alias = "name")]
    name: String,
    #[arg(long = "scopes", alias = "scopes")]
    scopes: String,
    #[arg(long = "transport", alias = "transport", default_value = "http")]
    transport: String,
}

#[derive(Debug, Args)]
struct RevokeTokenArgs {
    #[arg(
        long = "config",
        short = 'c',
        alias = "config",
        default_value = "config.yaml"
    )]
    config: String,
    #[arg(long = "name", alias = "name")]
    name: String,
}

#[derive(Debug, Subcommand)]
enum DbAction {
    Check(ConfigArg),
    Migrate(MigrateArgs),
    SyncSqlite(SyncSqliteArgs),
}

#[derive(Debug, Args)]
struct ConfigArg {
    #[arg(
        long = "config",
        short = 'c',
        alias = "config",
        default_value = "config.yaml"
    )]
    config: String,
}

#[derive(Debug, Args)]
struct MigrateArgs {
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    apply: bool,
    #[arg(
        long = "config",
        short = 'c',
        alias = "config",
        default_value = "config.yaml"
    )]
    config: String,
}

#[derive(Debug, Args)]
struct SyncSqliteArgs {
    #[arg(long = "source", default_value = "data/blog.db")]
    source: String,
    #[arg(
        long = "config",
        short = 'c',
        alias = "config",
        default_value = "config.yaml"
    )]
    config: String,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse_from(normalized_args());
    match cli.command {
        Command::ServeWeb(args) => {
            let cfg = config::load(&args.config)?;
            let pool = connect_checked_existing(&cfg).await?;
            let addr = format!("0.0.0.0:{}", cfg.server.port);
            let router = app::router_with_pool_and_config(
                pool,
                "public/assets",
                cfg.upload.dir.clone(),
                cfg,
            );
            let listener = TcpListener::bind(&addr).await?;
            println!("web server started addr={addr}");
            axum::serve(listener, router).await?;
            Ok(())
        }
        Command::ServeMcp(args) => {
            let cfg = config::load(&args.config)?;
            let pool = connect_checked_existing(&cfg).await?;
            match args.transport.to_ascii_lowercase().as_str() {
                "http" => {
                    let addr = cfg.mcp.http_addr.clone();
                    let router = blogweb::mcp::router_with_pool_and_config(pool, cfg);
                    let listener = TcpListener::bind(&addr).await?;
                    println!("mcp http server started addr={addr}");
                    axum::serve(listener, router).await?;
                    Ok(())
                }
                "stdio" => {
                    let stdin = std::io::stdin();
                    let stdout = std::io::stdout();
                    let stderr = std::io::stderr();
                    blogweb::mcp::serve_stdio(pool, cfg, stdin.lock(), stdout.lock(), stderr.lock())
                        .await
                }
                other => Err(blogweb::error::AppError::Config(format!(
                    "unsupported mcp transport {other}"
                ))),
            }
        }
        Command::Mcp(command) => match command.action {
            McpAction::IssueToken(args) => {
                if args.name.trim().is_empty() || args.scopes.trim().is_empty() {
                    return Err(blogweb::error::AppError::Config(
                        "name and scopes are required".into(),
                    ));
                }
                let cfg = config::load(&args.config)?;
                let pool = connect_checked_existing(&cfg).await?;
                let scopes = args
                    .scopes
                    .split(',')
                    .map(str::to_string)
                    .collect::<Vec<String>>();
                let token =
                    blogweb::mcp::issue_token(&pool, &cfg, &args.name, &scopes, &args.transport)
                        .await?;
                println!("name={}", args.name);
                println!("transport={}", args.transport);
                println!("token={token}");
                Ok(())
            }
            McpAction::RevokeToken(args) => {
                if args.name.trim().is_empty() {
                    return Err(blogweb::error::AppError::Config("name is required".into()));
                }
                let cfg = config::load(&args.config)?;
                let pool = connect_checked_existing(&cfg).await?;
                blogweb::mcp::revoke_token(&pool, &args.name).await
            }
        },
        Command::Db(command) => match command.action {
            DbAction::Check(args) => {
                let cfg = config::load(&args.config)?;
                connect_checked_existing(&cfg).await?;
                println!("database schema is ready");
                Ok(())
            }
            DbAction::Migrate(args) => {
                if args.dry_run == args.apply {
                    return Err(blogweb::error::AppError::Config(
                        "choose exactly one of --dry-run or --apply".into(),
                    ));
                }
                let cfg = config::load(&args.config)?;
                if args.dry_run {
                    let pool = db::connect_memory().await?;
                    db::apply_migrations(&pool).await?;
                    println!("database migration dry-run succeeded");
                    return Ok(());
                }
                let pool = db::connect(&cfg.database.url).await?;
                db::apply_migrations(&pool).await?;
                println!("database migration applied");
                Ok(())
            }
            DbAction::SyncSqlite(args) => {
                let cfg = config::load(&args.config)?;
                let pool = db::connect(&cfg.database.url).await?;
                db::apply_migrations(&pool).await?;
                let report = blogweb::sqlite_sync::sync_file(&args.source, &pool).await?;
                println!("sqlite source synced source={}", args.source);
                for (table, count) in report.table_counts() {
                    println!("{table}={count}");
                }
                Ok(())
            }
        },
    }
}

async fn connect_checked_existing(cfg: &config::Config) -> Result<db::DbPool> {
    let pool = db::connect_existing(&cfg.database.url).await?;
    db::check_migrations(&pool).await?;
    Ok(pool)
}

fn normalized_args() -> Vec<OsString> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if args.len() == 1 {
        let mut normalized = args;
        normalized.push(OsString::from("serve-web"));
        return normalized;
    }
    if args
        .get(1)
        .and_then(|arg| arg.to_str())
        .is_some_and(|arg| arg.starts_with('-'))
    {
        let mut normalized = Vec::with_capacity(args.len() + 1);
        normalized.push(args[0].clone());
        normalized.push(OsString::from("serve-web"));
        normalized.extend(args.into_iter().skip(1));
        return normalized;
    }
    args.into_iter().map(normalize_go_style_flag).collect()
}

fn normalize_go_style_flag(arg: OsString) -> OsString {
    match arg.to_str() {
        Some("-config") => OsString::from("--config"),
        Some("-name") => OsString::from("--name"),
        Some("-scopes") => OsString::from("--scopes"),
        Some("-transport") => OsString::from("--transport"),
        _ => arg,
    }
}
