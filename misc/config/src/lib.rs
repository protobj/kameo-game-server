use cfg::Tables;
use luban_lib::ByteBuf;
use ractor::{async_trait, ActorProcessingErr, ActorRef};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Arc, OnceLock, RwLock, RwLockReadGuard};

static TABLES: AtomicPtr<Arc<Tables>> = AtomicPtr::new(std::ptr::null_mut());
fn load(aws_config: &AwsConfig) {
    tracing::info!("load_config....");

    let tables = _load(aws_config);
    let ptr = Arc::into_raw(Arc::new(tables)) as *mut Arc<Tables>;
    TABLES.store(ptr, Ordering::Release);
}

fn _load(aws_config: &AwsConfig) -> Tables {
    let tables = Tables::new(|name| {
        let path = PathBuf::from(format!(
            "/home/cc/RustroverProjects/luban_examples/Projects/GenerateDatas/bytes/{}.bytes",
            name
        ));
        Ok(ByteBuf::new(std::fs::read(path).unwrap()))
    });
    tables.expect("REASON")
}
fn reload(aws_config: &AwsConfig) {
    tracing::info!("update_config....");
    let new_tables = _load(aws_config);
    let new_ptr = Arc::into_raw(Arc::new(new_tables)) as *mut Arc<Tables>;
    let mut old_ptr = TABLES.load(Ordering::Acquire);
    loop {
        match TABLES.compare_exchange_weak(old_ptr, new_ptr, Ordering::Release, Ordering::Relaxed) {
            Ok(_) => {
                // 安全释放旧配置
                if !old_ptr.is_null() {
                    unsafe { Arc::from_raw(old_ptr) };
                }
                break;
            }
            Err(e) => old_ptr = e,
        }
    }
}

pub fn get_config() -> Arc<Tables> {
    let ptr = TABLES.load(Ordering::Acquire);
    assert!(!ptr.is_null(), "Config not initialized");
    unsafe { Arc::clone(&*ptr) }
}
pub struct ConfigActor {}

pub enum ConfigActorMessage {
    Load,   //加载配置
    Reload, //重新加载配置
}
#[derive(Debug, Clone, Default)]
pub struct AwsConfig {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
    pub bucket: String,
    pub endpoint: String,
}
#[derive(Debug, Clone)]
pub struct ConfigActorArgument {
    pub aws_config: AwsConfig,
}

impl ractor::Message for ConfigActorMessage {}
#[async_trait]
impl ractor::Actor for ConfigActor {
    type Msg = ConfigActorMessage;
    type State = ();
    type Arguments = ConfigActorArgument;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }
    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ConfigActorMessage::Load => load(&AwsConfig::default()),
            ConfigActorMessage::Reload => reload(&AwsConfig::default()),
        }
        Ok(())
    }
}
