use common::config::ServerRoleId;
use kameo::actor::ActorID;
use kameo::error::{ActorStopReason, Infallible, RemoteSendError};
use kameo::remote::{SwarmBehaviour, SwarmRequest, SwarmResponse};
use libp2p::kad::store::RecordStore;
use libp2p::request_response::{OutboundRequestId, ResponseChannel};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{Multiaddr, gossipsub, kad, mdns, request_response, SwarmBuilder};
use libp2p_identity::PeerId;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::time::Duration;
use tracing::instrument::WithSubscriber;

pub(crate) async fn start_actor_swarm(
    server_role_id: ServerRoleId,
    self_address: String,
    other_addresses: Vec<String>,
) {

}

#[derive(NetworkBehaviour)]
pub(crate) struct ActorNodeBehaviour {
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
    request_response: request_response::cbor::Behaviour<SwarmRequest, SwarmResponse>,
    mdns: mdns::tokio::Behaviour,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ActorNodeMessage {
    Up(Cow<'static, str>),
    Down(Cow<'static, str>),
}

impl SwarmBehaviour for ActorNodeBehaviour {
    fn ask(
        &mut self,
        peer: &PeerId,
        actor_id: ActorID,
        actor_remote_id: Cow<'static, str>,
        message_remote_id: Cow<'static, str>,
        payload: Vec<u8>,
        mailbox_timeout: Option<Duration>,
        reply_timeout: Option<Duration>,
        immediate: bool,
    ) -> OutboundRequestId {
        self.request_response.send_request(
            peer,
            SwarmRequest::Ask {
                actor_id,
                actor_remote_id,
                message_remote_id,
                payload,
                mailbox_timeout,
                reply_timeout,
                immediate,
            },
        )
    }

    fn tell(
        &mut self,
        peer: &PeerId,
        actor_id: ActorID,
        actor_remote_id: Cow<'static, str>,
        message_remote_id: Cow<'static, str>,
        payload: Vec<u8>,
        mailbox_timeout: Option<Duration>,
        immediate: bool,
    ) -> OutboundRequestId {
        self.request_response.send_request(
            peer,
            SwarmRequest::Tell {
                actor_id,
                actor_remote_id,
                message_remote_id,
                payload,
                mailbox_timeout,
                immediate,
            },
        )
    }

    fn link(
        &mut self,
        actor_id: ActorID,
        actor_remote_id: Cow<'static, str>,
        sibbling_id: ActorID,
        sibbling_remote_id: Cow<'static, str>,
    ) -> OutboundRequestId {
        self.request_response.send_request(
            actor_id.peer_id().expect("swarm should be bootstrapped"),
            SwarmRequest::Link {
                actor_id,
                actor_remote_id,
                sibbling_id,
                sibbling_remote_id,
            },
        )
    }

    fn unlink(
        &mut self,
        actor_id: ActorID,
        actor_remote_id: Cow<'static, str>,
        sibbling_id: ActorID,
    ) -> OutboundRequestId {
        self.request_response.send_request(
            actor_id.peer_id().unwrap(),
            SwarmRequest::Unlink {
                actor_id,
                actor_remote_id,
                sibbling_id,
            },
        )
    }

    fn signal_link_died(
        &mut self,
        dead_actor_id: ActorID,
        notified_actor_id: ActorID,
        notified_actor_remote_id: Cow<'static, str>,
        stop_reason: ActorStopReason,
    ) -> OutboundRequestId {
        self.request_response.send_request(
            notified_actor_id.peer_id().unwrap(),
            SwarmRequest::SignalLinkDied {
                dead_actor_id,
                notified_actor_id,
                notified_actor_remote_id,
                stop_reason,
            },
        )
    }

    fn send_ask_response(
        &mut self,
        channel: ResponseChannel<SwarmResponse>,
        result: Result<Vec<u8>, RemoteSendError<Vec<u8>>>,
    ) -> Result<(), SwarmResponse> {
        self.request_response
            .send_response(channel, SwarmResponse::Ask(result))
    }

    fn send_tell_response(
        &mut self,
        channel: ResponseChannel<SwarmResponse>,
        result: Result<(), RemoteSendError<Vec<u8>>>,
    ) -> Result<(), SwarmResponse> {
        self.request_response
            .send_response(channel, SwarmResponse::Tell(result))
    }

    fn send_link_response(
        &mut self,
        channel: ResponseChannel<SwarmResponse>,
        result: Result<(), RemoteSendError<Infallible>>,
    ) -> Result<(), SwarmResponse> {
        self.request_response
            .send_response(channel, SwarmResponse::Link(result))
    }

    fn send_unlink_response(
        &mut self,
        channel: ResponseChannel<SwarmResponse>,
        result: Result<(), RemoteSendError<Infallible>>,
    ) -> Result<(), SwarmResponse> {
        self.request_response
            .send_response(channel, SwarmResponse::Unlink(result))
    }

    fn send_signal_link_died_response(
        &mut self,
        channel: ResponseChannel<SwarmResponse>,
        result: Result<(), RemoteSendError<Infallible>>,
    ) -> Result<(), SwarmResponse> {
        self.request_response
            .send_response(channel, SwarmResponse::SignalLinkDied(result))
    }

    fn kademlia_add_address(&mut self, peer: &PeerId, address: Multiaddr) -> kad::RoutingUpdate {
        self.kademlia.add_address(peer, address)
    }

    fn kademlia_set_mode(&mut self, mode: Option<kad::Mode>) {
        self.kademlia.set_mode(mode)
    }

    fn kademlia_get_record(&mut self, key: kad::RecordKey) -> kad::QueryId {
        self.kademlia.get_record(key)
    }

    fn kademlia_get_record_local(&mut self, key: &kad::RecordKey) -> Option<Cow<'_, kad::Record>> {
        self.kademlia.store_mut().get(key)
    }

    fn kademlia_put_record(
        &mut self,
        record: kad::Record,
        quorum: kad::Quorum,
    ) -> Result<kad::QueryId, kad::store::Error> {
        self.kademlia.put_record(record, quorum)
    }

    fn kademlia_put_record_local(&mut self, record: kad::Record) -> Result<(), kad::store::Error> {
        self.kademlia.store_mut().put(record)
    }

    fn kademlia_remove_record(&mut self, key: &kad::RecordKey) {
        self.kademlia.remove_record(key);
    }

    fn kademlia_remove_record_local(&mut self, key: &kad::RecordKey) {
        self.kademlia.store_mut().remove(key);
    }
}
