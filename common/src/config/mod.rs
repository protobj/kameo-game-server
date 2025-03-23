use anyhow::anyhow;
use clap::ArgAction;
use clap::builder::{TypedValueParser, ValueParserFactory};
use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::fs;
use std::io::{IsTerminal, stderr};
use std::path::{Path, PathBuf};
use std::str::FromStr;
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
    #[arg(short, long,action = ArgAction::Append,value_parser = parse_role_id)]
    pub server: Vec<ServerRoleId>,
    #[arg(short, long, default_value = "info", use_value_delimiter = true)]
    pub log: Vec<Directive>,
}
#[derive(Clone, Debug)]
pub struct ServerRoleId(pub ServerRole, pub u32);

impl FromStr for ServerRoleId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid format: {}, expected name/id", s));
        }

        let id = parts[1]
            .parse::<u32>()
            .map_err(|_| format!("Invalid ID: {}", parts[1]))?;
        let role = ServerRole::from_str(parts[0], true)
            .map_err(|_| format!("Invalid role: {}", parts[0]))?;
        Ok(ServerRoleId(role, id))
    }
}
// 自定义解析函数
fn parse_role_id(s: &str) -> Result<ServerRoleId, String> {
    ServerRoleId::from_str(s)
}
#[derive(Debug, Deserialize)]
pub struct GlobalConfig {
    config: DataConfig,
    login: Vec<LoginServerConfig>,
    gate: Vec<GateServerConfig>,
    world: Vec<WorldServerConfig>,
    game: Vec<GameServerConfig>,
}
impl From<&Args> for anyhow::Result<GlobalConfig> {
    fn from(value: &Args) -> Self {
        tracing::info!("conf file is : {}", &value.config);
        let config = fs::read_to_string(&value.config)?;
        let result: GlobalConfig = toml::from_str(&config)?;
        Ok(result)
    }
}
impl GlobalConfig {
    pub fn find_gate_config(&self, id: u32) -> Option<GateServerConfig> {
        return self.gate.iter().find(|g| g.id == id).cloned();
    }
    pub fn find_login_config(&self, id: u32) -> Option<LoginServerConfig> {
        return self.login.iter().find(|g| g.id == id).cloned();
    }
    pub fn find_world_config(&self, id: u32) -> Option<WorldServerConfig> {
        return self.world.iter().find(|g| g.id == id).cloned();
    }
    pub fn find_player_config(&self, id: u32) -> Option<GameServerConfig> {
        return self.game.iter().find(|g| g.id == id).cloned();
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
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
    pub in_address: String,
    pub keydb: RedisConfig,
}
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GateServerConfig {
    pub id: u32,
    pub in_address: String,
    pub out_tcp_port: u16,
    pub out_ws_port: u16,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct WorldServerConfig {
    pub id: u32,
    pub in_address: String,
}
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GameServerConfig {
    pub id: u32,
    pub in_address: String,
    pub keydb: RedisConfig,
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
