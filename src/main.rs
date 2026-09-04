use std::path::PathBuf;
use std::sync::Arc;
use clap::{Parser, Subcommand};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use episod::{
    create_router, AppState, BackendReplica, ConsistentHashRouter, EpisodConfig, InMemoryStore,
    OpenAIAdapter, ToolRegistry,
};

#[derive(Parser, Debug)]
#[command(name = "episod")]
#[command(author = "Episod Community")]
#[command(version = "0.1.0")]
#[command(about = "Stateful agentic gateway and episodic memory for LLM inference", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the Episod gateway server
    Serve {
        /// Path to configuration file (TOML)
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Port to bind the server to (overrides config)
        #[arg(short, long)]
        port: Option<u16>,

        /// Host to bind the server to (overrides config)
        #[arg(long)]
        host: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "episod=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { config, port, host } => {
            let mut cfg = if let Some(config_path) = config {
                let content = std::fs::read_to_string(&config_path)?;
                EpisodConfig::from_toml(&content)?
            } else {
                EpisodConfig::default()
            };

            if let Some(p) = port {
                cfg.server.port = p;
            }
            if let Some(h) = host {
                cfg.server.host = h;
            }

            // 1. Storage
            let store = Arc::new(InMemoryStore::new());

            // 2. Router with upstreams
            let router = ConsistentHashRouter::new(50);
            for up in &cfg.upstreams {
                info!("Adding upstream replica: {} -> {}", up.id, up.url);
                router.add_replica(BackendReplica::new(
                    up.id.clone(),
                    up.url.clone(),
                    up.weight,
                ));
            }

            // 3. Adapter & Tools
            let api_key = std::env::var("EPISOD_UPSTREAM_API_KEY").ok();
            let adapter = Arc::new(OpenAIAdapter::new(api_key));
            let tool_registry = ToolRegistry::new();

            // 4. AppState with default guardrails
            let state = AppState::new(
                store,
                router,
                adapter,
                tool_registry,
            );

            let app = create_router(state);

            let bind_addr = format!("{}:{}", cfg.server.host, cfg.server.port);
            let listener = TcpListener::bind(&bind_addr).await?;
            info!("⚡ Episod Gateway listening on http://{}", bind_addr);
            info!("  - Health check: http://{}/health", bind_addr);
            info!("  - Episodes API: http://{}/v1/episodes", bind_addr);

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        }
    }

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C signal handler");
    info!("Shutting down Episod gateway gracefully...");
}
