use crate::node::Node;
use common::config::{GlobalConfig, LoginServerConfig, ServerRoleId};
use kameo::actor::ActorRef;
use kameo::{Actor, RemoteActor};
use redis::aio::ConnectionManager;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

pub struct LoginNode {
    global_config: Arc<GlobalConfig>,
    role_id: ServerRoleId,
    login_ref: Option<ActorRef<LoginActor>>,
}
impl LoginNode {
    pub fn new(global_config: Arc<GlobalConfig>, role_id: ServerRoleId) -> Self {
        Self {
            global_config,
            role_id,
            login_ref: None,
        }
    }
}
#[async_trait::async_trait]
impl Node for LoginNode {
    async fn start(&mut self) -> anyhow::Result<()> {
        let global_config = self.global_config.clone();
        let role_id = self.role_id.clone();
        let login_config = global_config.find_login_config(role_id.1);
        let login_config = match login_config {
            None => return Err(anyhow::anyhow!("Login config not found:{}", role_id)),
            Some(x) => x,
        };

        self.start_actor_swarm(
            login_config.in_address.clone(),
            vec![global_config.center_in_address().to_string()],
        )
            .await?;

        //连接redis
        let redis_conn = common::redis::create(&login_config.keydb).await?;

        //集群启动好后,启动LoginActor
        let login_ref = kameo::spawn(LoginActor::new(
            global_config,
            role_id,
            login_config,
            redis_conn,
        ));
        let result = login_ref.wait_startup_result().await;
        if let Err(e) = result {
            return Err(anyhow::anyhow!(
                "LoginActor:{} start failed:{}",
                self.server_role_id(),
                e.to_string()
            ));
        };
        self.login_ref = Some(login_ref);
        self.connect_center().await?;
        tracing::info!("LoginActor start success:{}", self.role_id);
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        let actor_ref = self.login_ref.take().unwrap();
        //停止actor
        actor_ref.kill();
        actor_ref.wait_for_stop().await;
        Ok(())
    }

    fn server_role_id(&self) -> ServerRoleId {
        self.role_id.clone()
    }
}

#[derive(RemoteActor)]
pub struct LoginActor {
    config: Arc<GlobalConfig>,
    server_role_id: ServerRoleId,
    login_config: LoginServerConfig,
    pub redis_conn: ConnectionManager,
}

impl LoginActor {
    pub fn new(
        config: Arc<GlobalConfig>,
        server_role_id: ServerRoleId,
        login_config: LoginServerConfig,
        redis_conn: ConnectionManager,
    ) -> Self {
        Self {
            config,
            server_role_id,
            login_config,
            redis_conn,
        }
    }
}

impl Actor for LoginActor {
    type Error = LoginActorError;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), Self::Error> {
        actor_ref
            .register(self.server_role_id.to_string().as_str())
            .await
            .map_err(|e| {
                tracing::error!("LoginActor register remote fail:{}", e);
                LoginActorError::RegisterRemoteFail(e.to_string())
            })?;

        Ok(())
    }
}
#[derive(Debug, Clone)]
pub enum LoginActorError {
    RegisterRemoteFail(String),
}
impl Display for LoginActorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("LoginActorError")
    }
}

