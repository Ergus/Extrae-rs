#![allow(dead_code)]

pub struct GlobalInfo {

    pub buffer_set: crate::bufferset::BufferSet,
    name_set: crate::nameset::NameSet,

    thread_event_id: u16,
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

        let mut name_set = crate::nameset::NameSet::new();
        let buffer_set = crate::bufferset::BufferSet::new(
            start_system_time,
            trace_directory_path
        );

        let thread_event_id = name_set.register_event_name_internal("ThreadRuning");

        Self {
            buffer_set,
            name_set,
            thread_event_id
        }
    }

    fn finalize(&self) {

        //assert_eq!(self.buffer_set.thread_running, 0);

        println!("Finalizing profiler");

        let output_path = self.buffer_set.trace_directory_path.as_path();

	self.buffer_set
            .create_row(output_path)
            .expect("Error creatiiing ROW file");
        self.name_set
            .create_pcf(output_path)
            .expect("Error creating PCF file");

        println!("# Profiler TraceDir: {}", output_path.to_str().unwrap());
    }

}

// Here I will use an option in order to allow lazy initialization.
// This will force me to use unsafe code, but at leats I won't need to
// have a global Mutex for every access to the global information.
// The internal functions already have a lock when needed.
static mut INFO: Option<GlobalInfo> = None;

impl GlobalInfo {

    /// Get a buffer for this thread.
    /// The buffer may be created now or maybe recovered from a
    /// previous save.
    // This requires mutable access to the variable.
    pub(crate) fn get_buffer(tid: std::thread::ThreadId) -> crate::buffer::Buffer
    {
        unsafe {
            INFO.get_or_insert_with(|| GlobalInfo::new()).buffer_set.get_buffer(tid)
        }
    }

    // This requires mutable access to the variable.
    pub(crate) fn save_buffer_id(mut buffer: crate::buffer::Buffer)
    {
        buffer.flush().expect("Failed to flush buffer to file");
        unsafe {
            let remaining_threads = INFO.as_mut()
                .expect("Global info not set when called save_buffer_id")
                .buffer_set
                .save_buffer_id(&buffer);

            // remaining_threads is zero when the main thread exits,
            // so it is the last and we can exit.
            if remaining_threads == 0 {
                INFO.as_ref().unwrap().finalize();
            }
        }
    }

    pub fn register_event_name(
        event_name: &str,
        file_name: &str,
        line: u32,
        event: u16
    ) {
        unsafe {
            INFO.get_or_insert_with(|| GlobalInfo::new())
                .name_set
                .register_event_name(event_name, file_name, line, event);
        }
    }
}










