use anyhow::Ok;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use kameo::{
    Actor,
    mailbox::unbounded::UnboundedMailbox,
    message::{Message, StreamMessage},
};
use message::{GateServerMessage, LogicMessage};
use message_io::{
    network::{Endpoint, NetEvent, SendStatus, Transport},
    node::{self, NodeEvent, NodeHandler, NodeTask},
};
use tokio::sync::mpsc::{self, UnboundedReceiver};
use tokio_stream::wrappers::UnboundedReceiverStream;

use super::message;

struct GateNetServer {
    node: NodeHandler<()>,
    config: GateNetConfig,
    rx: Option<UnboundedReceiver<GateServerMessage>>,
    task: NodeTask,
}

pub struct GateNetConfig {
    pub tcp_port: u16,
    pub udp_port: u16,
    pub ws_port: u16,
}
impl GateNetServer {
    pub fn new(config: GateNetConfig) -> Option<Self> {
        let (node, listener) = node::split::<()>();

        let network_interface = "0.0.0.0";
        if node
            .network()
            .listen(Transport::FramedTcp, (network_interface, config.tcp_port))
            .is_err()
        {
            tracing::error!("Can not run server on TCP port {}", config.tcp_port);
            return None;
        }

        if node
            .network()
            .listen(Transport::Udp, (network_interface, config.udp_port))
            .is_err()
        {
            tracing::error!("Can not run server on UDP port {}", config.udp_port);
            return None;
        }
        if node
            .network()
            .listen(Transport::Ws, (network_interface, config.ws_port))
            .is_err()
        {
            tracing::error!("Can not run server on WS port {}", config.udp_port);
            return None;
        }

        tracing::info!(
            "Server running on ports {} (tcp) and {} (udp) and {} (ws)",
            config.tcp_port,
            config.udp_port,
            config.ws_port
        );
        let (tx, rx) = mpsc::unbounded_channel::<GateServerMessage>();
        let task = listener.for_each_async(move |event| match event {
            NodeEvent::Signal(signal) => unreachable!(),
            NodeEvent::Network(network) => match network {
                NetEvent::Connected(endpoint, _) => {
                    tx.send(GateServerMessage::Connected(endpoint)).unwrap();
                }
                NetEvent::Disconnected(endpoint) => {
                    tx.send(GateServerMessage::Disconnected(endpoint)).unwrap();
                }
                NetEvent::Message(endpoint, data) => {
                    tx.send(GateServerMessage::MessageRead(
                        endpoint,
                        LogicMessage::from(data),
                    ))
                    .unwrap();
                }
                NetEvent::Accepted(endpoint, _) => {
                    tx.send(GateServerMessage::Accepted(endpoint)).unwrap();
                }
            },
        });
        Some(Self {
            node,
            config,
            rx: Some(rx),
            task,
        })
    }
}

impl Actor for GateNetServer {
    type Mailbox = UnboundedMailbox<Self>;

    type Error = anyhow::Error;

    async fn on_start(
        &mut self,
        actor_ref: kameo::actor::ActorRef<Self>,
    ) -> Result<(), Self::Error> {
        let rx: UnboundedReceiver<GateServerMessage> = self.rx.take().unwrap();
        let stream = UnboundedReceiverStream::new(rx);
        actor_ref.attach_stream(stream, (), ());
        Ok(())
    }
}

impl Message<StreamMessage<GateServerMessage, (), ()>> for GateNetServer {
    type Reply = ();
    async fn handle(
        &mut self,
        msg: StreamMessage<GateServerMessage, (), ()>,
        ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            StreamMessage::Next(gate_server_message) => match gate_server_message {
                GateServerMessage::Connected(endpoint) => {
                    tracing::info!("{} connected", endpoint);
                }
                GateServerMessage::MessageRead(endpoint, message) => {
                    tracing::info!("{} MessageRead", endpoint);
                }
                GateServerMessage::Accepted(endpoint) => {
                    tracing::info!("{} Accepted", endpoint);
                }
                GateServerMessage::Disconnected(endpoint) => {
                    tracing::info!("{} Disconnected", endpoint);
                }
                GateServerMessage::MessageWrite(endpoint, message) => {
                    let bytes = LogicMessage::to_bytes(&message);
                    let send_status = self.node.network().send(endpoint, &bytes);
                    tracing::info!("{} MessageWrite status:{:?}", endpoint, send_status);
                }
            },
            StreamMessage::Started(_) => {
                tracing::info!("server start");
            }
            StreamMessage::Finished(_) => {
                tracing::info!("server stop");
            }
        }
    }
}
