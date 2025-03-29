use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u128 {
    let now = SystemTime::now();
    let duration_since_epoch = now.duration_since(UNIX_EPOCH).unwrap();
    duration_since_epoch.as_millis()
}
