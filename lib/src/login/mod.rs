use crate::Node;
use common::config::{GlobalConfig, LoginServerConfig, ServerRoleId};
use kameo::Actor;
use kameo::actor::ActorRef;
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
            global_config.find_all_in_address(),
        )
        .await?;
        //集群启动好后,启动LoginActor
        let login_ref = kameo::spawn(LoginActor::new(global_config, role_id, login_config));
        let result = login_ref.wait_startup_result().await;
        if let Err(e) = result {
            return Err(anyhow::anyhow!(
                "LoginActor:{} start failed:{}",
                self.server_role_id(),
                e.to_string()
            ));
        };
        self.login_ref = Some(login_ref);
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
pub struct LoginActor {
    config: Arc<GlobalConfig>,
    server_role_id: ServerRoleId,
    login_config: LoginServerConfig,
}

impl LoginActor {
    pub fn new(
        config: Arc<GlobalConfig>,
        server_role_id: ServerRoleId,
        login_config: LoginServerConfig,
    ) -> Self {
        Self {
            config,
            server_role_id,
            login_config,
        }
    }
}

impl Actor for LoginActor {
    type Error = LoginActorError;
}
#[derive(Debug, Clone)]
pub enum LoginActorError {}
impl Display for LoginActorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("LoginActorError")
    }
}
