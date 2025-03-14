mod tcp;

use crate::network::tcp::start_tcp_server;
use bytes::Bytes;
use std::net::SocketAddr;
use kameo::message::{Context, Message};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::mpsc::channel;

pub struct LogicMessage {
    pub ix: usize,
    pub cmd: u16,
    pub bytes: Bytes,
}

#[derive(Serialize,Deserialize)]
pub enum NetMessage {
    Read(LogicMessage),
    Connected(TcpStream, SocketAddr),
    Err,
    Close,
}

fn start_gate_server() {
    let (tx, mut rx) = channel::<NetMessage>(100);

    tokio::spawn(async move { start_tcp_server("0.0.0.0", 2322, tx) });

    tokio::select! {
       val = rx.recv() => {
            if let Some(msg) =val{
                match msg{
                    NetMessage::Msg(msg)=>{
                        println!("msg:{}",msg.ix);
                    },
                    NetMessage::Connected(stream,addr)=>{
                        println!("connected:{}",addr);
                    },
                    NetMessage::Err=>{
                        println!("err");
                    },
                    NetMessage::Close=>{
                        println!("close");
                    },
                }
            }
       }
    }
}
