#![allow(dead_code)]

use crate::Merger;

pub struct GlobalInfo {

    buffer_set: crate::bufferset::BufferSet,
    name_set: crate::nameset::NameSet,

    pub thread_event_id: u16,
}

impl GlobalInfo {
    fn new() -> Self
    {
        let start_system_time =
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards");

        // TODO: Convert the time to something friendly
        let trace_dir = format!("TRACEDIR_{}", start_system_time.as_millis());

        let trace_directory_path = std::path::PathBuf::from(trace_dir);
        std::fs::create_dir(&trace_directory_path)
            .unwrap_or_else(|_| {
                panic!("Failed to create trace directory: {}",
                    trace_directory_path.as_os_str()
                    .to_str()
                    .unwrap())
            });

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

    fn finalize(&self)
    {
        println!("Finalizing profiler");

        let output_path = self.buffer_set.trace_directory_path.as_path();

	self.buffer_set
            .create_row(output_path)
            .expect("Error creatiiing ROW file");
        self.name_set
            .create_pcf(output_path)
            .expect("Error creating PCF file");

        Merger::new(output_path)      // path to read from
            .create_prv(output_path)  // path to write to
            .expect("Error creating PRV file");

        println!("# Profiler TraceDir: {}", output_path.to_str().unwrap());
    }

}

// Here I will use an option in order to allow lazy initialization.
// This will force me to use unsafe code, but at leats I won't need to
// have a global Mutex for every access to the global information.
// The internal functions already have a lock when needed.
// The initialization happens in main and the risk of multiple attempts
// to initialize is very low, so we ignore it for now.
static mut INFO: Option<GlobalInfo> = None;

impl GlobalInfo {

    pub(crate) fn as_ref() -> &'static GlobalInfo
    {
        unsafe {
            INFO.get_or_insert_with(|| GlobalInfo::new()) as &GlobalInfo
        }
    }

    /// Get a buffer for this thread.
    /// The buffer may be created now or maybe recovered from a previous save.
    /// This requires mutable access to the variable.
    pub(crate) fn get_buffer(tid: std::thread::ThreadId) -> crate::buffer::Buffer
    {
        unsafe {
            INFO.get_or_insert_with(|| GlobalInfo::new()).buffer_set.get_buffer(tid)
        }
    }

    /// This requires mutable access to the variable.
    /// It saves the buffer id in the map set and discounts the running
    /// thread track variables.
    /// When the number of running threads is zero this also calls the
    /// finalize function to write all the output files and performs the
    /// merge+write
    pub(crate) fn notify_thread_finalized(buffer: &crate::buffer::Buffer)
    {
        unsafe {
            let remaining_threads = INFO
                .as_mut()
                .expect("Global info not set when called save_buffer_id")
                .buffer_set
                .save_buffer_id(&buffer);

            // remaining_threads is zero when the main thread is exiting,
            // so it is the last and we can exit.
            // The running threads counter is in the buffer_set for
            // not a very good reason
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
    ) -> u16 {
        unsafe {
            INFO.get_or_insert_with(|| GlobalInfo::new())
                .name_set
                .register_event_name(event_name, file_name, line, event)
        }
    }
}










