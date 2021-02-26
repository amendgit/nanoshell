use std::{rc::Rc, time::Duration};

use super::platform::run_loop::{
    HandleType, PlatformRunLoop, PlatformRunLoopSender, INVALID_HANDLE,
};

pub struct ScheduledCallback {
    platform_run_loop: Rc<PlatformRunLoop>,
    handle: HandleType,
}

impl ScheduledCallback {
    pub fn cancel(&mut self) {
        if self.handle != INVALID_HANDLE {
            self.platform_run_loop.unschedule(self.handle);

            self.handle = INVALID_HANDLE;
        }
    }

    pub fn detach(&mut self) {
        self.handle = INVALID_HANDLE;
    }
}

impl Drop for ScheduledCallback {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub struct RunLoop {
    platform_run_loop: Rc<PlatformRunLoop>,
}

impl RunLoop {
    pub fn new() -> Self {
        Self {
            platform_run_loop: Rc::new(PlatformRunLoop::new()),
        }
    }

    #[must_use]
    pub fn schedule<F>(&self, callback: F, in_time: Duration) -> ScheduledCallback
    where
        F: FnOnce() -> () + 'static,
    {
        ScheduledCallback {
            platform_run_loop: self.platform_run_loop.clone(),
            handle: self.platform_run_loop.schedule(callback, in_time),
        }
    }

    pub fn run(&self) {
        self.platform_run_loop.run()
    }

    pub fn stop(&self) {
        self.platform_run_loop.stop()
    }

    pub fn new_sender(&self) -> RunLoopSender {
        RunLoopSender {
            platform_sender: self.platform_run_loop.new_sender(),
        }
    }
}

// Can be used to send callbacks from other threads to be executed on run loop thread
pub struct RunLoopSender {
    platform_sender: PlatformRunLoopSender,
}

impl RunLoopSender {
    pub fn send<F>(&self, callback: F)
    where
        F: FnOnce() -> () + 'static + Send,
    {
        self.platform_sender.send(callback)
    }
}
