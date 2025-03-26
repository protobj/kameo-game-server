use network::LogicMessage;

pub struct GateSession {}

impl GateSession {
    pub(crate) async fn handle_message(&mut self, logic_message: LogicMessage) {}
}
