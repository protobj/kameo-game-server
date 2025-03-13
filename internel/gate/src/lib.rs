mod tcp;

use crossbeam_utils::sync::WaitGroup;
use ractor::rpc::cast;
use ractor::{Actor, ActorProcessingErr, ActorRef, Message, async_trait};
//ractor:
//   消息里可以传递ActorRef,
//   pg可以将Actor加到组里，发送消息给某一个组
//   register:
//        注册pid:集群内可用，任何位置都可以直接拿到actor，并且发送消息给他
//        注册name:不管是不是集群，只要在进程创建都会注册
//   actorruntime: 创建一个远程Actor,可以用来发消息，给远端的Actor
//

struct Gate;
struct GateState;
struct GateMessage;

impl Message for GateMessage {}

fn gate_main(wait_group: Option<WaitGroup>) {
    // if let Ok(mut wait_group) = wait_group {}
}
