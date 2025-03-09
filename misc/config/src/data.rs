
use arc_swap::{ArcSwap, ArcSwapAny, ArcSwapOption};
use aws_sdk_dynamodb::config::retry::ShouldAttempt::No;
use cfg::Tables;
use crossbeam::atomic::AtomicCell;
use crossbeam::epoch;
use crossbeam::epoch::{Atomic, Owned, Shared};
use luban_lib::ByteBuf;
use ractor::{ActorProcessingErr, ActorRef, async_trait};
use std::ops::Deref;
use std::path::PathBuf;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Arc, OnceLock, RwLock, RwLockReadGuard};

static TABLES: ArcSwapOption<Tables> = ArcSwapOption::const_empty();

pub fn load(aws_config: &AwsConfig) {
    tracing::info!("load_config....");

    let tables = _load(aws_config);
    TABLES.store(Some(Arc::new(tables)));
}

pub fn reload(aws_config: &AwsConfig) {
    tracing::info!("reload....");
    let new_tables = _load(aws_config);
    TABLES.swap(Some(Arc::new(new_tables)));
    tracing::info!("reloaded");
}

pub fn get() -> Arc<Tables> {
    TABLES.load().clone().unwrap()
}
fn _load(aws_config: &AwsConfig) -> Tables {
    let tables = Tables::new(|name| {
        let path = PathBuf::from(format!(
            "/home/cc/RustroverProjects/luban_examples/Projects/GenerateDatas/bytes/{}.bytes",
            name
        ));
        Ok(ByteBuf::new(std::fs::read(path).unwrap()))
    });
    tables.expect("luban err")
}

#[derive(Debug, Clone, Default)]
pub struct AwsConfig {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
    pub bucket: String,
    pub endpoint: String,
}
