use std::sync::atomic::Ordering;
use std::sync::{Arc, atomic::AtomicBool};
use std::thread;
use std::time::Duration;

pub struct Timeout {
    abort: Arc<AtomicBool>,
}

impl Timeout {
    pub fn set<F>(delay: Duration, callback: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let abort = Arc::new(AtomicBool::new(false));
        let abort_clone = abort.clone();

        thread::spawn(move || {
            thread::sleep(delay);
            if !abort_clone.load(Ordering::Relaxed) {
                callback();
            }
        });

        Timeout { abort }
    }

    pub fn clear(&mut self) {
        self.abort.store(true, Ordering::Relaxed);
    }
}
