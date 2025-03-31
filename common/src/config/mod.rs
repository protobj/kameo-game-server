use clap::ArgAction;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::fs;
use std::str::FromStr;
use std::sync::OnceLock;
pub static ARGS: OnceLock<Args> = OnceLock::new();

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
}
#[derive(Clone, Debug,Serialize, Deserialize,PartialEq,Eq,Hash)]
pub struct ServerRoleId(pub ServerRole, pub u32);
impl Display for ServerRoleId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{}-{}", self.0.to_string(), self.1.to_string()).as_str())
    }
}
impl FromStr for ServerRoleId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid format: {}, expected name-id", s));
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
    pub config: DataConfig,
    pub log: LogConfig,
    login: Vec<LoginServerConfig>,
    gate: Vec<GateServerConfig>,
    world: Vec<WorldServerConfig>,
    game: Vec<GameServerConfig>,
    center_in_address:String,
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
    pub fn find_game_config(&self, id: u32) -> Option<GameServerConfig> {
        return self.game.iter().find(|g| g.id == id).cloned();
    }

    pub fn find_all_login_config(&self) -> Vec<LoginServerConfig> {
        return self.login.clone();
    }
    pub fn find_all_in_address(&self) -> Vec<String> {
        let mut addresses = vec![];
        self.login.iter().for_each(|g| {
            addresses.push(g.in_address.clone());
        });
        self.world.iter().for_each(|g| {
            addresses.push(g.in_address.clone());
        });
        self.game.iter().for_each(|g| {
            addresses.push(g.in_address.clone());
        });
        self.gate.iter().for_each(|g| {
            addresses.push(g.in_address.clone());
        });
        addresses
    }

    pub fn login(&self) -> &Vec<LoginServerConfig> {
        &self.login
    }

    pub fn gate(&self) -> &Vec<GateServerConfig> {
        &self.gate
    }

    pub fn world(&self) -> &Vec<WorldServerConfig> {
        &self.world
    }

    pub fn game(&self) -> &Vec<GameServerConfig> {
        &self.game
    }

    pub fn center_in_address(&self) -> &str {
        &self.center_in_address
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LogConfig {
    pub console: bool,
    pub level: String,
    pub dir: String,
    pub max_file: u32,
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
    pub out_tcp_port: Option<u16>,
    pub out_ws_port: Option<u16>,
    pub out_udp_port: Option<u16>,
}
impl GateServerConfig {
    pub fn unique_name(&self) -> String {
        return format!("{}-{}", ServerRole::Gate, self.id);
    }
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
#[derive(Debug, Clone, ValueEnum, Deserialize,Serialize,Hash,Eq, PartialEq)]
pub enum ServerRole {
    Login,
    Gate,
    Game,
    World,
    Center,
}
impl Display for ServerRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerRole::Login => f.write_str("login"),
            ServerRole::Gate => f.write_str("gate"),
            ServerRole::Game => f.write_str("game"),
            ServerRole::World => f.write_str("world"),
            ServerRole::Center => f.write_str("center"),
        }
    }
}
pub mod config_test {
    use crate::config::{ARGS, Args, GlobalConfig, init_config};

    #[test]
    pub fn test() {
        init_config();
        let args = ARGS.get().expect("init fail");

        let config: GlobalConfig = <&Args as Into<anyhow::Result<GlobalConfig>>>::into(args)
            .expect("GlobalConfig init fail");

        tracing::info!("config:{:?}", config)
    }
}
