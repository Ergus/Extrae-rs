#![allow(dead_code)]

use crate::global_info::GlobalInfo; 

pub struct ThreadInfo {
    tid: std::thread::ThreadId,
    id: u32,
    buffer_events: crate::buffer::Buffer,
    events_manager: Option<crate::perf::PerfManager>,
}

impl ThreadInfo {

    fn new() -> Self
    {
        let thread: std::thread::Thread = std::thread::current();

        let tid = thread.id();
        let name = thread.name().unwrap_or_default();

        let mut buffer_events = GlobalInfo::get_thread_buffer(tid, name);
        let id = buffer_events.id();

        buffer_events.emplace_event(GlobalInfo::as_ref().thread_event_id, 1);

        let events_manager =
            crate::perf::PerfManager::new(&["cycles", "instructions"]);

        Self { tid, id, buffer_events, events_manager }
    }
}

impl Drop for ThreadInfo {

    fn drop(&mut self)
    {
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

    pub fn with<F, R>(f: F) -> R
    where
      F: FnOnce(&ThreadInfo) -> R,
    {
        // Just make a dummy reference for the case 
        ThreadInfo::THREAD_INFO.with(f)
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

    pub fn emplace_event_and_counters(id: u16, value: u32)
    {
        ThreadInfo::THREAD_INFO.with(|info| {
            let mut_info = info as *const ThreadInfo as *mut ThreadInfo;
            unsafe {

                match &mut (*mut_info).events_manager {
                    Some(manager) => {
                        let mut events = manager.get_counters();
                        events.push((id, value));
                        (*mut_info).buffer_events.emplace_events(&events);
                    },
                    None => (*mut_info).buffer_events.emplace_event(id, value),
                }
            }
        })
    }


}
