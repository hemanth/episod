use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use episod::{
    AppState, BackendReplica, ConsistentHashRouter, EpisodConfig, Episode, InMemoryStore,
    OpenAIAdapter, SqliteStore, StateStore, ToolRegistry, create_router, spawn_health_checker,
};

#[derive(Parser, Debug)]
#[command(name = "episod")]
#[command(author = "Episod Community")]
#[command(version = "0.1.0")]
#[command(about = "The stateful agentic gateway for LLM inference", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the Episod gateway server
    #[command(alias = "start")]
    Serve {
        /// Path to configuration file (TOML)
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Port to bind the server to (overrides config or $PORT)
        #[arg(short, long, env = "PORT")]
        port: Option<u16>,

        /// Host to bind the server to (overrides config or $HOST)
        #[arg(long, env = "HOST")]
        host: Option<String>,
    },

    /// Zero-config local development mode (auto-detects local Ollama / vLLM)
    Dev {
        /// Port to bind the server to (default 8080 or $PORT)
        #[arg(short, long, env = "PORT", default_value_t = 8080)]
        port: u16,

        /// Path to SQLite database
        #[arg(long, default_value = "episod.db")]
        db: String,
    },

    /// Visualize the conversational DAG tree for an episode
    Tree {
        /// Episode ID to inspect
        episode_id: String,

        /// Path to SQLite database
        #[arg(long, default_value = "episod.db")]
        db: String,
    },

    /// Benchmark KV-cache affinity latency savings (TTFT comparison)
    Bench {
        /// Target Episod gateway URL (e.g. http://localhost:8080)
        #[arg(short, long)]
        target: Option<String>,

        /// Number of conversation turns per session (default: 5)
        #[arg(short = 'n', long, default_value_t = 5)]
        turns: usize,

        /// Number of concurrent sessions (default: 4)
        #[arg(short = 's', long, default_value_t = 4)]
        sessions: usize,

        /// Force simulation mode against simulated GPU cluster
        #[arg(long)]
        simulated: bool,
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

            let env_port = std::env::var("PORT")
                .or_else(|_| std::env::var("EPISOD_PORT"))
                .ok()
                .and_then(|p| p.parse::<u16>().ok());

            if let Some(p) = port.or(env_port) {
                cfg.server.port = p;
            }
            if let Some(h) = host {
                cfg.server.host = h;
            }

            // 1. Storage
            let store: Arc<dyn StateStore> = match cfg.storage.storage_type.as_str() {
                "memory" => {
                    info!("Using In-Memory state store");
                    Arc::new(InMemoryStore::new())
                }
                "redis" => {
                    info!(
                        "Using Redis distributed state store: {}",
                        cfg.storage.redis_url
                    );
                    let redis_store = episod::RedisStore::new(&cfg.storage.redis_url)
                        .await?
                        .with_ttl(cfg.storage.ttl_seconds);
                    Arc::new(redis_store)
                }
                "http" => {
                    let url = cfg
                        .storage
                        .endpoint_url
                        .or_else(|| std::env::var("EPISOD_STORAGE_URL").ok())
                        .unwrap_or_else(|| "http://127.0.0.1:3000/api/episodes".to_string());
                    let auth = cfg
                        .storage
                        .auth_header
                        .or_else(|| std::env::var("EPISOD_STORAGE_AUTH").ok());
                    info!("Using HTTP remote webhook state store: {}", url);
                    Arc::new(episod::HttpStore::new(url, auth))
                }
                _ => {
                    info!(
                        "Using SQLite persistent state store: {}",
                        cfg.storage.database_path
                    );
                    Arc::new(SqliteStore::new(&cfg.storage.database_path)?)
                }
            };

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

            // Spawn background health checker (polls every 5s)
            spawn_health_checker(router.clone(), Duration::from_secs(5));

            // 3. Adapter & Tools
            let api_key = std::env::var("EPISOD_UPSTREAM_API_KEY").ok();
            let adapter = Arc::new(OpenAIAdapter::new(api_key));
            let tool_registry = ToolRegistry::new();

            // 4. AppState with default guardrails & striped session locks
            let state = AppState::new(store, router, adapter, tool_registry);

            let app = create_router(state);

            let bind_addr = format!("{}:{}", cfg.server.host, cfg.server.port);
            let listener = TcpListener::bind(&bind_addr).await?;
            println!();
            println!("  ⚡ Episod Gateway Running on http://{}", bind_addr);
            println!("  -------------------------------------------------------------");
            println!("  • Web Inspector:    http://{}/dashboard", bind_addr);
            println!("  • Health Endpoint:  http://{}/health", bind_addr);
            println!("  • Episodes API:     http://{}/v1/episodes", bind_addr);
            println!("  • OpenAI Responses: http://{}/v1/responses", bind_addr);
            println!(
                "  • Chat Completions: http://{}/v1/chat/completions",
                bind_addr
            );
            println!("  -------------------------------------------------------------");
            println!();

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        }

        Commands::Dev { port, db } => {
            println!();
            println!("  ⚡ Starting Episod in Zero-Config Dev Mode...");
            println!("  -------------------------------------------------------------");

            // Auto-detect local LLM runtimes
            let client = reqwest::Client::builder()
                .timeout(Duration::from_millis(800))
                .build()
                .unwrap_or_default();

            let mut detected_url = "http://127.0.0.1:8000/v1".to_string();
            let mut detected_id = "default-upstream".to_string();

            if let Ok(resp) = client.get("http://127.0.0.1:11434/api/tags").send().await {
                if resp.status().is_success() {
                    detected_url = "http://127.0.0.1:11434/v1".to_string();
                    detected_id = "local-ollama".to_string();
                    println!("  🔍 Auto-detected local engine: Ollama on http://127.0.0.1:11434");
                }
            } else if let Ok(resp) = client.get("http://127.0.0.1:8000/v1/models").send().await {
                if resp.status().is_success() {
                    detected_url = "http://127.0.0.1:8000/v1".to_string();
                    detected_id = "local-vllm".to_string();
                    println!("  🔍 Auto-detected local engine: vLLM on http://127.0.0.1:8000");
                }
            } else {
                println!(
                    "  ℹ️  No local engine auto-detected. Defaulting upstream to http://127.0.0.1:8000/v1"
                );
            }

            println!("  💾 Persistent Store: SQLite (WAL mode) -> {}", db);

            let store: Arc<dyn StateStore> = Arc::new(SqliteStore::new(&db)?);
            let router = ConsistentHashRouter::new(50);
            router.add_replica(BackendReplica::new(detected_id, detected_url, 1));
            spawn_health_checker(router.clone(), Duration::from_secs(5));

            let api_key = std::env::var("EPISOD_UPSTREAM_API_KEY").ok();
            let adapter = Arc::new(OpenAIAdapter::new(api_key));
            let tool_registry = ToolRegistry::new();

            let state = AppState::new(store, router, adapter, tool_registry);

            let app = create_router(state);
            let bind_addr = format!("127.0.0.1:{}", port);
            let listener = TcpListener::bind(&bind_addr).await?;

            println!("  ⚡ Gateway listening on http://{}", bind_addr);
            println!("  -------------------------------------------------------------");
            println!("  • Web Inspector:    http://{}/dashboard", bind_addr);
            println!("  • OpenAI Responses: http://{}/v1/responses", bind_addr);
            println!(
                "  • Chat Completions: http://{}/v1/chat/completions",
                bind_addr
            );
            println!("  -------------------------------------------------------------");
            println!();

            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        }

        Commands::Tree { episode_id, db } => {
            let store = SqliteStore::new(&db)?;
            match store.get_episode(&episode_id).await? {
                Some(ep) => print_dag_tree(&ep),
                None => {
                    eprintln!("❌ Episode '{}' not found in database '{}'", episode_id, db);
                    std::process::exit(1);
                }
            }
        }

        Commands::Bench {
            target,
            turns,
            sessions,
            simulated,
        } => {
            episod::bench::run_benchmark(episod::bench::BenchmarkConfig {
                target,
                turns,
                sessions,
                simulated,
            })
            .await?;
        }
    }

    Ok(())
}

fn print_dag_tree(ep: &Episode) {
    println!();
    println!("🌿 Episode: {} (Model: {})", ep.id, ep.model);
    if let Some(ref sys) = ep.system_prompt {
        println!("│  ⚙️ System Prompt: \"{}\"", sys.replace('\n', " "));
    }
    println!("│");

    if ep.nodes.is_empty() {
        println!("└── (No turns recorded yet)");
        println!();
        return;
    }

    // Sort turns chronologically
    let mut turns: Vec<_> = ep.nodes.values().collect();
    turns.sort_by_key(|n| n.created_at);

    for (idx, node) in turns.iter().enumerate() {
        let is_last = idx == turns.len() - 1;
        let branch_char = if is_last { "└──" } else { "├──" };
        let leaf_marker = if ep.active_leaf_id.as_deref() == Some(&node.id) {
            " 🌿 [Active Leaf]"
        } else {
            ""
        };

        println!(
            "{} [{}] Node ID: {}{}",
            branch_char,
            idx + 1,
            node.id,
            leaf_marker
        );

        if let Some(ref parent) = node.parent_id {
            println!("│   ↳ Parent: {}", parent);
        }

        if let Some(ref u) = node.user_message {
            let content = u.content.as_deref().unwrap_or("");
            let snippet: String = content.chars().take(80).collect();
            println!("│   👤 User: \"{}\"", snippet.replace('\n', " "));
        }

        for tc in &node.tool_calls {
            println!(
                "│   🛠️ Tool Call: {}(args: {})",
                tc.function.name, tc.function.arguments
            );
        }

        for tr in &node.tool_results {
            let out_snippet: String = tr.output.chars().take(80).collect();
            println!("│   ⚙️ Tool Result: \"{}\"", out_snippet.replace('\n', " "));
        }

        if let Some(ref a) = node.assistant_message {
            let content = a.content.as_deref().unwrap_or("");
            let snippet: String = content.chars().take(80).collect();
            println!("│   🤖 Assistant: \"{}\"", snippet.replace('\n', " "));
        }

        let ttft = node
            .timing
            .as_ref()
            .and_then(|t| t.ttft_ms)
            .map(|ms| format!("{}ms", ms))
            .unwrap_or_else(|| "n/a".into());
        let total = node
            .timing
            .as_ref()
            .map(|t| format!("{}ms", t.total_ms))
            .unwrap_or_else(|| "n/a".into());
        let tokens = node
            .usage
            .as_ref()
            .map(|u| format!("{} tok", u.total_tokens))
            .unwrap_or_else(|| "n/a".into());
        let hit_rate = node
            .usage
            .as_ref()
            .and_then(|u| u.cache_hit_rate)
            .map(|r| format!("{:.0}%", r * 100.0))
            .unwrap_or_else(|| "n/a".into());

        println!(
            "│   📊 Metrics: TTFT: {} | Total: {} | Tokens: {} | Cache: {}",
            ttft, total, tokens, hit_rate
        );
        if !is_last {
            println!("│");
        }
    }
    println!();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C signal handler");
    info!("Shutting down Episod gateway gracefully...");
}
