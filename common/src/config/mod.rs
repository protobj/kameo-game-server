use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::fs;
use std::io::{IsTerminal, stderr};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use toml::Table;
use tracing_subscriber::filter::Directive;

static ARGS: OnceLock<Args> = OnceLock::new();

pub fn init_config() {
    ARGS.get_or_init(|| Args::parse());
}
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "../conf/config-dev.toml")]
    pub config: String,
    #[arg(short, long, value_enum, default_value = "gate")]
    pub role: ServerRole,
    #[arg(short, long, default_value = "1")]
    pub id: u32,
    #[arg(short, long, default_value = "info", use_value_delimiter = true)]
    pub log: Vec<Directive>,
}

impl Args {
    fn get_server_id(&self) -> String {
        return format!("{:?}/{}", &self.role, &self.id);
    }
}
#[derive(Debug, Deserialize)]
pub struct GlobalConfig {
    config: DataConfig,
    login: Vec<LoginServerConfig>,
    gate: Vec<GateServerConfig>,
    world: Vec<WorldServerConfig>,
    player: Vec<PlayerServerConfig>,
}
impl From<&Args> for anyhow::Result<GlobalConfig> {
    fn from(value: &Args) -> Self {
        tracing::info!("conf file is : {}", &value.config);
        let config = fs::read_to_string(&value.config)?;
        let mut result: GlobalConfig = toml::from_str(&config)?;
        Ok(result)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: Option<String>,
    pub db: i64,
    pub pool_size: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DataConfig {
    pub r#type: ConfigType,
    pub source_type: ConfigSourceType,
}
#[derive(Debug, Clone, Deserialize)]
pub enum ConfigType {
    #[serde(rename = "bin")]
    Bin,
}
impl Default for ConfigType {
    fn default() -> Self {
        ConfigType::Bin
    }
}
#[derive(Debug, Clone, Deserialize)]
pub enum ConfigSourceType {
    #[serde(rename = "aws")]
    Aws {
        access_key_id: String,
        secret_access_key: String,
        region: String,
        bucket: String,
        endpoint: String,
    },
}
impl Default for ConfigSourceType {
    fn default() -> Self {
        ConfigSourceType::Aws {
            access_key_id: String::default(),
            secret_access_key: String::default(),
            region: String::default(),
            bucket: String::default(),
            endpoint: String::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LoginServerConfig {
    pub id: u32,
    pub host: String,
    pub port: u16,
}
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GateServerConfig {
    pub id: u32,
    pub host: String,
    pub port: u16,
    pub out_tcp_port: u16,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct WorldServerConfig {
    pub id: u32,
    pub host: String,
    pub port: u16,
}
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PlayerServerConfig {
    pub id: u32,
    pub host: String,
    pub port: u16,
}
#[derive(Debug, Clone, ValueEnum, Deserialize)]
pub enum ServerRole {
    Login,
    Gate,
    Game,
    World,
}
pub mod config_test {
    use crate::config::{ARGS, Args, GlobalConfig, init_config};
    use crate::logging::init_logging;

    #[test]
    pub fn test() {
        init_config();
        let args = ARGS.get().expect("init fail");
        init_logging(args.log.clone());

        let config: GlobalConfig = <&Args as Into<anyhow::Result<GlobalConfig>>>::into(args)
            .expect("GlobalConfig init fail");

        tracing::info!("config:{:?}", config)
    }
}
