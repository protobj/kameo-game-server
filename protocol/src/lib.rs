pub mod base {
    include!(concat!(env!("OUT_DIR"), "/base.rs"));
}
pub mod store_cmd {
    include!(concat!(env!("OUT_DIR"), "/store_cmd.rs"));
}
pub mod login_cmd {
    include!(concat!(env!("OUT_DIR"), "/login_cmd.rs"));
}
pub mod login {
    include!(concat!(env!("OUT_DIR"), "/login.rs"));
}
pub mod store {
    include!(concat!(env!("OUT_DIR"), "/store.rs"));
}
pub mod base_cmd {
    include!(concat!(env!("OUT_DIR"), "/base_cmd.rs"));
}
pub mod stream {
    include!(concat!(env!("OUT_DIR"), "/stream.rs"));
}
pub mod snapshot {
    include!(concat!(env!("OUT_DIR"), "/snapshot.rs"));
}
pub mod extension;
