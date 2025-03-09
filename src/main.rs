use clap::Parser;
use config::Args;
use lazy_static::lazy_static;
use log::trace;
use ractor::{cast, Actor, ActorProcessingErr, ActorRef, Message, SupervisionEvent};
use std::env;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    ::logging::init_logging(args.log.clone());
    tracing::info!("args:{:?}", args);
    tracing::info!("my id {}",config::cluster::get_server_id(&args.role, &args.id));

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
