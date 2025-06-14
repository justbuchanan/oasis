use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::signal::Signal;

// Context object used to cancel an async operation and optionally wait for it to complete.
pub struct CancelContext {
    cancel_signal: Signal<NoopRawMutex, ()>,
    done_signal: Signal<NoopRawMutex, ()>,
}

impl Default for CancelContext {
    fn default() -> Self {
        Self::new()
    }
}

impl CancelContext {
    pub fn new() -> Self {
        Self {
            cancel_signal: Signal::new(),
            done_signal: Signal::new(),
        }
    }

    pub fn cancel(&self) {
        self.cancel_signal.signal(());
    }

    pub async fn cancel_and_wait(&self) {
        self.cancel();
        self.done_signal.wait().await
    }

    pub async fn wait_for_cancel(&self) {
        self.cancel_signal.wait().await
    }

    pub fn done(&self) {
        self.done_signal.signal(());
    }
}
