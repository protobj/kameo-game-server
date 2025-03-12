use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, async_trait};
use ractor_cluster::node::{NodeConnectionMode, NodeServerSessionInformation};
use ractor_cluster::{NodeEventSubscription, NodeServer, NodeServerMessage, RactorClusterMessage};

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
        match message {
            Self::Msg::Rpc(request, reply) => {
                let reply_msg = format!("reply {request}.");
                reply.send(reply_msg.clone())?;
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    ::logging::init_logging(vec!["info".parse().unwrap()]);
    let actor = NodeServer::new(
        9090,
        "cookie".to_string(),
        "Gate/2".to_string(),
        "locahost".to_string(),
        None,
        Some(NodeConnectionMode::Transitive),
    );

    let (actorRef, handle) = Actor::spawn(Some("Gate/2".to_string()), actor, ())
        .await
        .unwrap();

    struct Listener;
    impl NodeEventSubscription for Listener {
        fn node_session_opened(&self, ses: NodeServerSessionInformation) {
            println!("node_session_opened")
        }

        fn node_session_disconnected(&self, ses: NodeServerSessionInformation) {
            println!("node_session_disconnected")
        }

        fn node_session_authenicated(&self, ses: NodeServerSessionInformation) {
            println!("node_session_authenicated")
        }
    }
    actorRef
        .send_message(NodeServerMessage::SubscribeToEvents {
            id: "111".to_string(),
            subscription: Box::new(Listener),
        })
        .expect("TODO: panic message");

    let (test_actor, test_handle) =
        Actor::spawn(Some("LoginPingPongActor".to_string()), PingPongActor, ())
            .await
            .expect("Ping pong actor failed to start up!");

    test_handle.await.unwrap();
    handle.await.unwrap();
}
