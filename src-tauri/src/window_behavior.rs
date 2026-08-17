use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub struct WindowBehavior {
    auto_hide_suspended: AtomicBool,
}

impl WindowBehavior {
    pub fn set_auto_hide_suspended(&self, suspended: bool) {
        self.auto_hide_suspended.store(suspended, Ordering::Relaxed);
    }

    pub fn auto_hide_suspended(&self) -> bool {
        self.auto_hide_suspended.load(Ordering::Relaxed)
    }
}
