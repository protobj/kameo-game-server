pub mod cmd {
    include!(concat!(env!("OUT_DIR"), "/cmd.rs"));
}
pub mod login {
    include!(concat!(env!("OUT_DIR"), "/login.rs"));
}
pub mod store {
    include!(concat!(env!("OUT_DIR"), "/store.rs"));
}

pub mod node {
    include!(concat!(env!("OUT_DIR"), "/node.rs"));
}