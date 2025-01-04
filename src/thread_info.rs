#![allow(dead_code)]

use crate::global_info::GlobalInfo; 

pub struct ThreadInfo {
    tid: std::thread::ThreadId,
    id: u32,
    is_main: bool,
    buffer_events: crate::buffer::Buffer,
}

impl ThreadInfo {

    fn new() -> Self
    {
        let thread: std::thread::Thread = std::thread::current();

        let tid = thread.id();
        let is_main = thread.name().is_some_and(|x| x == "main");

        let mut buffer_events = GlobalInfo::get_thread_buffer(tid);
        let id = buffer_events.id();

        buffer_events.emplace_event(GlobalInfo::as_ref().thread_event_id, 1);

        Self { tid, id, is_main, buffer_events }
    }
}

impl Drop for ThreadInfo {
    fn drop(&mut self) {
        self.buffer_events.emplace_event(GlobalInfo::as_ref().thread_event_id, 0);
        self.buffer_events.flush().expect("Failed to flush buffer data");
        GlobalInfo::notify_thread_finalized(&self.buffer_events);
    }
}

impl ThreadInfo {

    // Use thread_local to define a thread-local storage
    thread_local! {
        static THREAD_INFO: ThreadInfo = ThreadInfo::new();
    }

    pub fn is_main() -> bool
    {
        ThreadInfo::THREAD_INFO.with(|info| {info.is_main})

    }

    pub fn emplace_event(id: u16, value: u32)
    {
        ThreadInfo::THREAD_INFO.with(|info| {
            let mut_info = info as *const ThreadInfo as *mut ThreadInfo;
            unsafe {
                (*mut_info).buffer_events.emplace_event(id, value);
            }
        })
    }
}
