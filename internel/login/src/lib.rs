use crossbeam_utils::sync::WaitGroup;
pub async fn login_main(wg: Option<WaitGroup>) {
}

struct LoginServer;

enum LoginServerMessage {
    PlayerLogin { id: String },
    GateMessage(),

}

struct LoginServerState {}

#[derive(Debug, Clone, Default)]
struct PlayerLoginGate2LoginReq {
    session_id: i32,
    msg: LoginInfo,
}
#[derive(Debug, Clone, Default)]
struct LoginInfo {
    account_type: i32,
    account: String,
    token: String,
    user_id: i64,
    local: UserLocalInfo,
    server_id: i32,
    user_name: String,
}
#[derive(Debug, Clone, Default)]
struct UserLocalInfo {
    ip: String,
    device_name: String,
}
