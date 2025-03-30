use anyhow::Error;
use common::config::{init_config, Args, GlobalConfig, ServerRole, ServerRoleId, ARGS};
use common::logging::init_log;
use lib::node::{Node, Signal};
use lib::prelude::{GameNode, GateNode, LoginNode, WorldNode};
use std::sync::Arc;
use tokio::sync::watch;
use tokio::sync::watch::Sender;
use tokio::task::JoinHandle;

pub async fn start() -> Result<(), Error> {
    //1.初始化命令行参数
    init_config();
    let args = match ARGS.get() {
        None => return Err(anyhow::anyhow!("args error")),
        Some(x) => x,
    };

    //2.读配置文件
    let config: GlobalConfig = <&Args as Into<anyhow::Result<GlobalConfig>>>::into(args)?;

    //3.启动日志输出
    let log_name = if args.server.len() > 1 {
        String::from("all")
    } else if args.server.len() <= 0 {
        return Err(anyhow::anyhow!("set server error"));
    } else {
        let x = args.server.get(0).unwrap();
        x.to_string()
    };
    let _log_guards = init_log(config.log.clone(), log_name)?;
    tracing::info!("config:{:?}", config);

    //发送退出信号
    let (tx, rx) = watch::channel(Signal::None);
    let config = Arc::new(config);
    //启动节点
    let mut join_handles = vec![];
    for server_role_id in &args.server {
        let server_role_id = server_role_id.clone();
        let config_clone = config.clone();
        let signal_rx = rx.clone();

        let mut node: Box<dyn Node> =
            create_node(&server_role_id.0, config_clone, server_role_id.clone());
        let join_handle = tokio::spawn(async move {
            let full_name = server_role_id.to_string();
            node.init(signal_rx)
                .await
                .expect(format!("init {} failed", full_name).as_str());
        });
        join_handles.push(join_handle)
    }

    listen_stop(tx, &mut join_handles);

    tracing::info!("server starting");
    let result = futures::future::join_all(join_handles).await;
    tracing::info!("server shutting down");
    for x in result {
        if let Err(e) = x {
            tracing::error!("error on stop:{}", e);
        }
    }
    tracing::info!("server is down");
    Ok(())
}

fn listen_stop(tx: Sender<Signal>, join_handles: &mut Vec<JoinHandle<()>>) {
    join_handles.push(tokio::spawn(async move {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };
        #[cfg(unix)]
        let terminate = async {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
            sigterm.recv().await;
        };
        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                tracing::info!("shutting down on ctrl-c handler");
                tx.send(Signal::Stop).expect("failed to send signal:Stop");
            },
            _ = terminate => {
                tracing::info!("shutting down on termination handler");
                tx.send(Signal::Stop).expect("failed to send signal:Stop");
            },
        }
    }));
}

fn create_node(
    role: &ServerRole,
    config: Arc<GlobalConfig>,
    server_role_id: ServerRoleId,
) -> Box<dyn Node> {
    match role {
        ServerRole::Login => Box::new(LoginNode::new(config, server_role_id)),
        ServerRole::Gate => Box::new(GateNode::new(config, server_role_id)),
        ServerRole::Game => Box::new(GameNode::new(config, server_role_id)),
        ServerRole::World => Box::new(WorldNode::new(config, server_role_id)),
    }
}
