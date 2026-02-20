use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mininq", about = "SQLite-backed job runner")]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "mininq.toml")]
    pub config: PathBuf,

    /// Server host
    #[arg(long, env = "MININQ_HOST")]
    pub host: Option<String>,

    /// Server port
    #[arg(long, env = "MININQ_PORT")]
    pub port: Option<u16>,

    /// Database path
    #[arg(long, env = "MININQ_DB_PATH")]
    pub db_path: Option<String>,

    /// Log level
    #[arg(long, env = "MININQ_LOG_LEVEL")]
    pub log_level: Option<String>,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub worker: WorkerConfig,
    #[serde(default)]
    pub defaults: DefaultsConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_db_path")]
    pub db_path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WorkerConfig {
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
    #[serde(default)]
    pub queues: Vec<String>,
    #[serde(default = "default_reaper_interval_secs")]
    pub reaper_interval_secs: u64,
    #[serde(default = "default_scheduler_interval_secs")]
    pub scheduler_interval_secs: u64,
    #[serde(default)]
    pub retention_days: Option<u32>,
    #[serde(default = "default_cleanup_interval_secs")]
    pub cleanup_interval_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DefaultsConfig {
    #[serde(default = "default_max_retries")]
    pub max_retries: i32,
    #[serde(default = "default_retry_backoff")]
    pub retry_backoff: String,
    #[serde(default = "default_base_delay_ms")]
    pub base_delay_ms: i32,
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: i32,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    6390
}
fn default_db_path() -> String {
    "mininq.db".to_string()
}
fn default_concurrency() -> usize {
    10
}
fn default_poll_interval_ms() -> u64 {
    500
}
fn default_reaper_interval_secs() -> u64 {
    30
}
fn default_scheduler_interval_secs() -> u64 {
    15
}
fn default_cleanup_interval_secs() -> u64 {
    3600
}
fn default_max_retries() -> i32 {
    3
}
fn default_retry_backoff() -> String {
    "exponential".to_string()
}
fn default_base_delay_ms() -> i32 {
    1000
}
fn default_max_delay_ms() -> i32 {
    300000
}
fn default_timeout_ms() -> i32 {
    30000
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "json".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            db_path: default_db_path(),
        }
    }
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            concurrency: default_concurrency(),
            poll_interval_ms: default_poll_interval_ms(),
            queues: Vec::new(),
            reaper_interval_secs: default_reaper_interval_secs(),
            scheduler_interval_secs: default_scheduler_interval_secs(),
            retention_days: None,
            cleanup_interval_secs: default_cleanup_interval_secs(),
        }
    }
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            retry_backoff: default_retry_backoff(),
            base_delay_ms: default_base_delay_ms(),
            max_delay_ms: default_max_delay_ms(),
            timeout_ms: default_timeout_ms(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

pub fn load_config(cli: &Cli) -> anyhow::Result<Config> {
    let mut config = if cli.config.exists() {
        let content = std::fs::read_to_string(&cli.config)?;
        toml::from_str(&content)?
    } else {
        Config::default()
    };

    // CLI args override config file
    if let Some(ref host) = cli.host {
        config.server.host = host.clone();
    }
    if let Some(port) = cli.port {
        config.server.port = port;
    }
    if let Some(ref db_path) = cli.db_path {
        config.server.db_path = db_path.clone();
    }
    if let Some(ref level) = cli.log_level {
        config.logging.level = level.clone();
    }

    Ok(config)
}
