#![allow(dead_code)]

use crate::global_info::GlobalInfo; 

pub struct ThreadInfo {
    id: u32,
    tid: std::thread::ThreadId,
    buffer_events: crate::buffer::Buffer,
}

impl ThreadInfo {

    fn new() -> Self
    {
        let tid = std::thread::current().id();
        let mut buffer_events = GlobalInfo::get_buffer(tid);
        let id = buffer_events.id();

        buffer_events.emplace_event(GlobalInfo::as_ref().thread_event_id, 1);

        Self { tid, id, buffer_events }
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
