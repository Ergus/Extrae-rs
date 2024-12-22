#![allow(dead_code)]

use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::thread::ThreadId;

use crate::buffer;
use crate::global_info::GlobalInfo;

pub struct BufferSet {
    events_map: Arc<RwLock<HashMap<ThreadId, buffer::Buffer>>>,
    thread_counter: u32,
}

impl BufferSet {

    pub fn new() -> Self
    {
        Self {
            events_map: Arc::new(RwLock::new(HashMap::new())),
            thread_counter: 0
        }
    }

    pub fn get_buffer(&mut self, tid: std::thread::ThreadId) -> buffer::Buffer
    {
        // We attempt to take the read lock first. If this tid was
        // already used, the buffer must be already created, and we
        // don't need the exclusive access.
        let mut mapwrite  = self.events_map.write().expect("Failed to get events_map lock");

        match mapwrite.entry(tid) {
            Entry::Occupied(entry) => entry.remove(),
            Entry::Vacant(_) => {

                self.thread_counter += 1;

                let filename = GlobalInfo::get_info()
                    .trace_directory_path
                    .join(format!("Trace_{}", self.thread_counter));

                buffer::Buffer::new(
                    self.thread_counter,
                    &tid,
                    filename,
                    &GlobalInfo::get_info().start_system_time
                )
            }
        }
    }

    pub fn save_buffer(&mut self, buffer: buffer::Buffer)
    {
        // We attempt to take the read lock first. If this tid was
        // already used, the buffer must be already created, and we
        // don't need the exclusive access.
        let mut mapwrite  = self.events_map.write().expect("Failed to get events_map lock");

        match mapwrite.entry(buffer.tid()) {
            Entry::Occupied(_) => panic!("Error reinserting buffer for existing tid"),
            Entry::Vacant(entry) => {
                entry.insert(buffer);
            }
        }
    }
}
