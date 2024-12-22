#![allow(dead_code)]

use crate::global_info::GlobalInfo; 

struct ThreadInfo {
    id: u32,
    tid: std::thread::ThreadId,
    buffer_events: crate::buffer::Buffer,
}

impl ThreadInfo {

    fn new() -> Self
    {
        let tid = std::thread::current().id();
        let buffer_events = GlobalInfo::get_buffer(tid);
        let id = buffer_events.id();

        Self { tid, id, buffer_events }
    }
}

impl ThreadInfo {

    // // Use thread_local to define a thread-local storage
    // thread_local! {
    //     static THREAD_INFO: ThreadInfo = ThreadInfo::new();
    // }


    // // Function to get a mutable reference to the thread-local InfoThread
    // fn get_info_thread() -> std::cell::RefMut<'static, ThreadInfo> {
    //     ThreadInfo::THREAD_INFO.with(|info| info.borrow_mut())
    // }

}
