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
use crate::config::ConfigSourceType;

static TABLES: ArcSwapOption<Tables> = ArcSwapOption::const_empty();

pub fn load(aws_config: &ConfigSourceType) {
    tracing::info!("load_config....");

    let tables = _load(aws_config);
    TABLES.store(Some(Arc::new(tables)));
}

pub fn reload(aws_config: &ConfigSourceType) {
    tracing::info!("reload....");
    let new_tables = _load(aws_config);
    TABLES.swap(Some(Arc::new(new_tables)));
    tracing::info!("reloaded");
}

pub fn get() -> Arc<Tables> {
    TABLES.load().clone().unwrap()
}
fn _load(aws_config: &ConfigSourceType) -> Tables {
    let tables = Tables::new(|name| {
        let path = PathBuf::from(format!(
            "/home/cc/RustroverProjects/luban_examples/Projects/GenerateDatas/bytes/{}.bytes",
            name
        ));
        Ok(ByteBuf::new(std::fs::read(path).unwrap()))
    });
    tables.expect("luban err")
}