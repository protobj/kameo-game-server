pub mod cmd {
    include!(concat!(env!("OUT_DIR"), "/cmd.rs"));
}
pub mod snapshot {
    include!(concat!(env!("OUT_DIR"), "/snapshot.rs"));
}
pub mod stream {
    include!(concat!(env!("OUT_DIR"), "/stream.rs"));
}
