pub mod cmd {
    include!(concat!(env!("OUT_DIR"), "/cmd.rs"));
}
pub mod common {
    include!(concat!(env!("OUT_DIR"), "/common.rs"));
}
pub mod hunt {
    include!(concat!(env!("OUT_DIR"), "/hunt.rs"));
}
pub mod alliance {
    include!(concat!(env!("OUT_DIR"), "/alliance.rs"));
}
pub mod login {
    include!(concat!(env!("OUT_DIR"), "/login.rs"));
}
pub mod tower {
    include!(concat!(env!("OUT_DIR"), "/tower.rs"));
}
pub mod charge {
    include!(concat!(env!("OUT_DIR"), "/charge.rs"));
}
pub mod stream {
    include!(concat!(env!("OUT_DIR"), "/stream.rs"));
}
pub mod snapshot {
    include!(concat!(env!("OUT_DIR"), "/snapshot.rs"));
}
