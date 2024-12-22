#![allow(dead_code)]

pub struct GlobalInfo {

    pub start_system_time: std::time::Duration,
    pub trace_directory_path: std::path::PathBuf,

    pub buffer_set: crate::bufferset::BufferSet,
    name_set: crate::nameset::NameSet,

    // thread_event_id: u16,
}

impl GlobalInfo {

    fn new() -> Self
    {
        let start_system_time =
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards");

        // TODO: Convert the time to something friendly
        let trace_dir = format!("TRACEDIR_{}", start_system_time.as_secs());

        let trace_directory_path = std::path::PathBuf::from(trace_dir);
        if !trace_directory_path.exists() {
            std::fs::create_dir(&trace_directory_path).expect("Failed to create trade directory");
        }

        let name_set = crate::nameset::NameSet::new();
        // let thread_event_id = name_set.register_event_name_internal("ThreadRuning");

        Self {
            start_system_time,
            trace_directory_path,
            buffer_set: crate::bufferset::BufferSet::new(),
            name_set
        }
    }
}

impl Drop for GlobalInfo {
    fn drop(&mut self) {
        
    }
}

static mut INFO: Option<GlobalInfo> = None;

impl GlobalInfo {

    // Borrow static immutable
    pub fn get_info() -> &'static GlobalInfo {
        unsafe {
            INFO.get_or_insert_with(|| GlobalInfo::new())
        }
    }

    // This requires mutable access to the variable.
    pub fn get_buffer(tid: std::thread::ThreadId) -> crate::buffer::Buffer
    {
        unsafe {
            INFO.get_or_insert_with(|| GlobalInfo::new()).buffer_set.get_buffer(tid)
        }
    }
}


