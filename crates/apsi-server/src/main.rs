use apsi_core::Engine;
use apsi_core::auth::{ActorKind, Principal, Role, TokenRegistry};
use apsi_science::ScienceRegistry;
use apsi_server::{AppState, router};
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

/// APSInsight API server.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Address to listen on.
    #[arg(long, env = "APSI_BIND", default_value = "127.0.0.1:8080")]
    bind: SocketAddr,
    /// Directory for persistent state; in-memory when omitted.
    #[arg(long, env = "APSI_DATA_DIR")]
    data_dir: Option<PathBuf>,
    /// Comma-separated `token:tenant:subject:role[+role]` entries.
    #[arg(long, env = "APSI_TOKENS", hide_env_values = true)]
    tokens: Option<String>,
    /// Built Angular app to serve (e.g. web/dist/web/browser).
    #[arg(long, env = "APSI_WEB_DIR")]
    web_dir: Option<PathBuf>,
    /// Requests per token per minute.
    #[arg(long, env = "APSI_RATE_LIMIT", default_value_t = 600)]
    rate_limit: u32,
    /// Development mode: generates demo tokens for tenant `demo` and prints them.
    #[arg(long)]
    dev: bool,
    /// Emit logs as JSON lines.
    #[arg(long, env = "APSI_LOG_JSON")]
    log_json: bool,
}

fn random_token() -> String {
    format!("apsi_{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args = Args::parse();
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=info".into());
    if args.log_json {
        tracing_subscriber::fmt().json().with_env_filter(filter).init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }

    let mut tokens = match args.tokens.as_deref().map(TokenRegistry::from_spec).transpose() {
        Ok(t) => t.unwrap_or_default(),
        Err(e) => {
            eprintln!("invalid --tokens: {e}");
            return std::process::ExitCode::from(2);
        }
    };
    if args.dev {
        eprintln!("development tokens for tenant `demo` (valid until restart):");
        for (subject, role) in [("admin", Role::Admin), ("editor", Role::Editor), ("viewer", Role::Viewer), ("auditor", Role::Auditor)] {
            let t = random_token();
            eprintln!("  {subject:8} {t}");
            tokens.insert(&t, Principal::new(subject, "demo", &[role], ActorKind::User));
        }
    }
    if tokens.is_empty() {
        eprintln!("no API tokens configured: pass --tokens / APSI_TOKENS, or --dev for local development");
        return std::process::ExitCode::from(2);
    }

    let engine = match &args.data_dir {
        Some(dir) => match Engine::open(dir) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("cannot open data dir {}: {e}", dir.display());
                return std::process::ExitCode::from(9);
            }
        },
        None => Engine::in_memory(),
    }
    .with_compute(Arc::new(ScienceRegistry));

    let app = router(AppState::new(engine, tokens, args.rate_limit), args.web_dir);
    let listener = match tokio::net::TcpListener::bind(args.bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot bind {}: {e}", args.bind);
            return std::process::ExitCode::from(2);
        }
    };
    tracing::info!(bind = %args.bind, persistent = args.data_dir.is_some(), "APSInsight API listening");
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown requested; draining connections");
    };
    match axum::serve(listener, app).with_graceful_shutdown(shutdown).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("server error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
