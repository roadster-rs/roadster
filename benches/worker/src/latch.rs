use std::{future::Future, sync::Arc};
use tokio::sync::Semaphore;

// https://github.com/tokio-rs/tokio/issues/6087
#[derive(Clone)]
pub struct Countdown(Arc<Semaphore>);

impl Countdown {
    pub fn new(n: u32) -> (Self, impl Future + Send) {
        let sem = Arc::new(Semaphore::new(0));
        let latch = Self(sem.clone());

        let wait = async move {
            let _ = sem.acquire_many(n).await;
        };

        (latch, wait)
    }

    pub fn count_down(&self) {
        self.0.add_permits(1);
    }
}
