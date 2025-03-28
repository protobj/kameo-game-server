use message_io::network::{NetEvent, Transport};
use message_io::node;

pub struct NetServer {

}

pub fn start_server(
    tcp_addr: Option<String>,
    ws_addr: Option<String>,
    udp_addr: Option<String>,
) -> Result<(), anyhow::Error> {
    let (handler, listener) = node::split::<()>();
    if let Some(addr) = tcp_addr {
        handler.network().listen(Transport::Tcp, addr)?;
    }
    if let Some(addr) = ws_addr {
        handler.network().listen(Transport::Ws, addr)?;
    }
    if let Some(addr) = udp_addr {
        handler.network().listen(Transport::Udp, addr)?;
    }

    let task = listener.for_each_async(move |event| match event.network() {
        NetEvent::Connected(_, _) => (), // Only generated at connect() calls.
        NetEvent::Accepted(endpoint, _listener_id) => {}
        NetEvent::Message(endpoint, input_data) => {}
        NetEvent::Disconnected(endpoint) => {}
    });
    Ok(())
}
