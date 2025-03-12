use clap::Parser;
use log::trace;
use ractor::{
    Actor, ActorId, ActorProcessingErr, ActorRef, Message, RpcReplyPort, SupervisionEvent,
    async_trait, cast,
};
use ractor_cluster::node::{NodeConnectionMode, NodeServerSessionInformation};
use ractor_cluster::{
    NodeEventSubscription, NodeServer, NodeServerMessage, NodeSession, RactorClusterMessage,
};
use std::env;
use std::thread::sleep;
use std::time::Duration;

struct PingPongActor;

#[derive(RactorClusterMessage)]
enum PingPongActorMessage {
    #[rpc]
    Rpc(String, RpcReplyPort<String>),
}
#[async_trait]
impl Actor for PingPongActor {
    type Msg = PingPongActorMessage;
    type State = ();
    type Arguments = ();

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _: (),
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let group = "Login".to_string();
        // let remote_actors = ActorRef::<Self::Msg>::from(ractor::pg::get_members(&group)[0].clone());

        let actor = ractor::registry::where_is("LoginPingPongActor".to_string());

        // let actorId = remote_actors.get_id();
        // let name = remote_actors.get_name();
        match message {
            Self::Msg::Rpc(request, reply) => {
                tracing::info!(
                    "Received an RPC of '{request}' replying in kind to {} remote actors",
                    1
                );
                let reply_msg = format!("{request}.");
                reply.send(reply_msg.clone())?;

                // let _reply = ractor::call_t!(
                //     remote_actors,
                //     PingPongActorMessage::Rpc,
                //     100,
                //     reply_msg.clone()
                // )?;
                // tracing::info!("reply from {:?} msg:{:?} {:?}", actorId, name, reply_msg);
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let args = common::config::Args::parse();
    //
    common::logging::init_logging(vec!["info".parse().unwrap()]);
    // tracing::info!("args:{:?}", args);
    // tracing::info!("my id {}",conf::cluster::get_server_id(&args.role, &args.id));
    let connect_client = Some(9090);
    let cookie = "cookie".to_string();
    let hostname = "localhost".to_string();

    let server =
        ractor_cluster::NodeServer::new(3030, cookie, "node_a".to_string(), hostname, None, Some(NodeConnectionMode::Transitive));

    let (actor, handle) = Actor::spawn(None, server, ())
        .await
        .expect("Failed to start NodeServer A");

    let (test_actor, test_handle) =
        Actor::spawn(Some("MainPingPongActor".to_string()), PingPongActor, ())
            .await
            .expect("Ping pong actor failed to start up!");

    if let Some(cport) = connect_client {
        if let Err(error) =
            ractor_cluster::node::client::connect(&actor, format!("127.0.0.1:{cport}")).await
        {
            tracing::error!("Failed to connect with error {error}")
        } else {
            tracing::info!("Client connected to NdoeServer");
        }
    }

    // wait for server startup to complete (and in the event of a client, wait for auth to complete), then startup the test actor
    ractor::concurrency::sleep(Duration::from_millis(1000)).await;
    // test_actor.cast(PingPongActorMessage::Ping).unwrap();
    let _ = test_actor
        .call(
            |tx| PingPongActorMessage::Rpc("firstPing".to_string(), tx),
            Some(Duration::from_millis(50)),
        )
        .await
        .unwrap();

    // wait for exit
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for event");

    // cleanup
    test_actor.stop(None);
    test_handle.await.unwrap();
    actor.stop(None);
    handle.await.unwrap();
}

//作为服务器的根，管理者角色
struct RootActor;

enum RootActorMessage {
    Close, //关闭服务器
}
impl Message for RootActorMessage {}
enum RootActorState {
    Starting,
    Start,
    Stopping,
    Stopped,
}
#[ractor::async_trait]
impl Actor for RootActor {
    type Msg = RootActorMessage;
    type State = RootActorState;
    type Arguments = ();

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("RootActor Starting {myself:?}");
        //启动ConfigActor

        //启动TcpActor,接受请求
        Ok(RootActorState::Start)
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        Ok(())
    }

    async fn post_stop(
        &self,
        myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        Ok(())
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            RootActorMessage::Close => {
                tracing::info!("RootActor Close");
                *state = RootActorState::Stopping;
                myself
                    .stop_children_and_wait(None, Some(Duration::from_secs(30)))
                    .await;
                myself.stop(None);
                *state = RootActorState::Stopped
            }
        }
        Ok(())
    }
    async fn handle_supervisor_evt(
        &self,
        myself: ActorRef<Self::Msg>,
        message: SupervisionEvent,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            SupervisionEvent::ActorStarted(actorCell) => {
                tracing::info!("ActorStarted：{actorCell:?}");
            }
            SupervisionEvent::ActorTerminated(cell, option, string) => {
                tracing::info!("ActorTerminated：{cell:?} {option:?} {string:?}");
            }
            SupervisionEvent::ActorFailed(actorCell, err) => {
                tracing::info!("ActorStarted：{actorCell:?} err:{err:?}");
            }
            SupervisionEvent::ProcessGroupChanged(groupChangeMessage) => {
                tracing::info!("ActorStarted：{groupChangeMessage:?}");
            }
            SupervisionEvent::PidLifecycleEvent(e) => {
                tracing::info!("ActorStarted：{e:?}");
            }
        }
        Ok(())
    }
}
