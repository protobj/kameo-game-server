use crate::cluster::{Cluster, Role};
use clap::Parser;
use tracing_subscriber::filter::Directive;

pub mod cluster;
mod data;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "./config/dev.toml")]
    pub config: String,
    #[arg(short, long, default_value = "./config/base.toml")]
    pub global_config: String,
    #[arg(short = 'd', long, default_value = "./config/cluster-dev.toml")]
    pub cluster_config: String,
    #[arg(short, long, value_enum)]
    pub role: Role,
    #[arg(short, long)]
    pub id: u32,
    #[arg(short, long, default_value = "info", use_value_delimiter = true)]
    pub log: Vec<Directive>,
}

#[derive(Debug)]
pub struct GlobalConfig {
    pub args: Args,
    pub cluster: Cluster,
}

#[derive(Debug)]
pub struct Database {
    pub mysql_url: String,
    pub redis_url: String,
}
