use crate::gate::client::{ClientActor, ClientMessage};
use crate::gate::packet::{Decoder, Encoder, Packet};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use kameo::Actor;
use kameo::actor::ActorRef;
use kameo::message::{Context, Message};
use message_io::network::{Endpoint, NetEvent, SendStatus, Transport};
use message_io::node;
use message_io::node::{NodeEvent, NodeHandler, NodeListener, NodeTask};
use scc::HashIndex;
use std::io;

pub struct NetServer {
    handler: NodeHandler<NetServerSignal>,
    node_listener: Option<NodeListener<NetServerSignal>>,
    client_actors: HashIndex<Endpoint, ActorRef<ClientActor>>,
}
pub enum NetServerSignal {
    CloseSession(Endpoint),
    SendPacket(Endpoint, Packet),
}
impl NetServer {
    pub fn new(
        tcp_port: Option<u16>,
        ws_port: Option<u16>,
        udp_port: Option<u16>,
    ) -> io::Result<NetServer> {
        let (handler, listener) = node::split::<NetServerSignal>();
        if let Some(port) = tcp_port {
            let addr = ("0.0.0.0", port);
            let result = handler.network().listen(Transport::FramedTcp, &addr)?;
            tracing::info!("tcp server listening on {:?}", result);
        }
        if let Some(port) = ws_port {
            let addr = ("0.0.0.0", port);
            let result = handler.network().listen(Transport::Ws, &addr)?;
            tracing::info!("ws server listening on {:?}", result);
        }
        if let Some(port) = udp_port {
            let addr = ("0.0.0.0", port);
            let result = handler.network().listen(Transport::Udp, &addr)?;
            tracing::info!("udp server listening on {:?}", result);
        }
        Ok(NetServer {
            handler,
            node_listener: Some(listener),
            client_actors: HashIndex::new(),
        })
    }

    pub fn run(mut self) -> (NodeTask, NodeHandler<NetServerSignal>) {
        let handler = self.handler.clone();
        let client_actors = self.client_actors.clone();
        let node_listener = self.node_listener.take().unwrap();
        let task = node_listener.for_each_async(move |event| {
            match event {
                NodeEvent::Network(net_event) => match net_event {
                    NetEvent::Connected(_, _) => (), // Only generated at connect() calls.
                    NetEvent::Accepted(endpoint, _listener_id) => {
                        let actor_ref = kameo::spawn(ClientActor::new(endpoint, handler.clone()));
                        client_actors.insert(endpoint, actor_ref.clone()).unwrap();
                    }
                    NetEvent::Message(endpoint, input_data) => {
                        let x = client_actors.peek_with(&endpoint, |_, v| v.clone());
                        if let Some(actor_ref) = x {
                            let x1 = if actor_ref.is_alive() {
                                let packet = Decoder::decode(input_data);
                                let packet = if let Some(packet) = packet { packet } else {
                                    tracing::error!("actor:{:?} decoding failed endpoint:{} input_data:{:?}",actor_ref, endpoint,input_data);
                                    handler.signals().send_with_priority(NetServerSignal::CloseSession(endpoint));
                                    return;
                                };

                                tokio::spawn(async move {
                                    let result = actor_ref.tell(ClientMessage::ReceivePacket(packet)).await;
                                    //发送失败
                                    if let Err(e) = result {
                                        tracing::warn!("actor:{:?} Failed to send message:{}", actor_ref,e);
                                    }
                                });
                            } else {
                                tracing::warn!("actor:{:?} is not alive:{} ", actor_ref ,endpoint);
                                handler.signals().send(NetServerSignal::CloseSession(endpoint));
                            };
                            x1
                        } else {
                            tracing::warn!("Received unexpected message from client,server already clean,but client still send message");
                        }
                    }
                    NetEvent::Disconnected(endpoint) => {
                        client_actors.remove(&endpoint);
                    }
                }
                NodeEvent::Signal(s) => match s {
                    NetServerSignal::CloseSession(s) => {
                        client_actors.remove(&s);
                    }
                    NetServerSignal::SendPacket(s, p) => {
                        let typ = p.r#type.to_string();
                        let send_status = handler.network().send(s, Encoder::encode(p).as_ref());
                        if send_status != SendStatus::Sent {
                            tracing::error!("endpoint:{:?} Failed to send packet: {:?}",s,typ);
                        };
                    }
                }
            }
        });
        (task, self.handler.clone())
    }
}
