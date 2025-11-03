use std::time::{Duration as StdDuration, Instant as StdInstant};

pub type Duration = StdDuration;
pub type Instant = StdInstant;

pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

pub fn init_logging(_level: log::Level) {}

pub fn install_panic_hook() {}
