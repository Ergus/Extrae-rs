#![allow(dead_code)]

use crate::global_info::GlobalInfo; 
use std::mem::ManuallyDrop;

pub struct ThreadInfo {
    id: u32,
    tid: std::thread::ThreadId,
    buffer_events: ManuallyDrop<crate::buffer::Buffer>,
}

impl ThreadInfo {

    fn new() -> Self
    {
        let tid = std::thread::current().id();
        let buffer = GlobalInfo::get_buffer(tid);
        let id = buffer.id();

        Self { tid, id, buffer_events:  ManuallyDrop::new(buffer) }
    }
}

impl Drop for ThreadInfo {
    fn drop(&mut self) {
        unsafe {
            GlobalInfo::save_buffer_id(ManuallyDrop::take(&mut self.buffer_events));
        }
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
