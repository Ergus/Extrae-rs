#![allow(dead_code)]

use std::sync::atomic;

use crate::{Merger,buffer};

pub struct GlobalInfo {

    buffer_set: crate::bufferset::BufferSet,
    name_set: crate::nameset::NameSet,

    threads_running: atomic::AtomicU32,

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

            threads_running: atomic::AtomicU32::new(0),

            thread_event_id
        }
    }

    fn init_buffer(&mut self, tid: std::thread::ThreadId, name: &str) -> buffer::Buffer
    {
        self.threads_running.fetch_add(1, atomic::Ordering::Relaxed);
        self.buffer_set.get_buffer(tid, name)
    }

    fn finalize_buffer(&mut self, buffer: &buffer::Buffer)
    {
        self.buffer_set.save_buffer_id(&buffer);
        self.threads_running.fetch_sub(1, atomic::Ordering::Relaxed);

        // Call finalize if this is the main thread.
        if buffer.name() == "main" {
            assert_eq!(self.threads_running.load(atomic::Ordering::Relaxed), 0);
            self.finalize();
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
    pub(crate) fn get_thread_buffer(tid: std::thread::ThreadId, name: &str) -> crate::buffer::Buffer
    {
        unsafe {
            INFO.get_or_insert_with(|| GlobalInfo::new())
        }.init_buffer(tid, name)
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
            INFO.as_mut()
                .expect("Global info not set when called save_buffer_id")
        }.finalize_buffer(buffer);
    }

    pub fn register_event_name(
        event_name: &str,
        file_name: Option<&str>,
        line: Option<u32>,
        event: Option<u16>
    ) -> u16 {
        unsafe {
            INFO.get_or_insert_with(|| GlobalInfo::new())
                .name_set
                .register_event_name(event_name, file_name, line, event)
        }
    }
}










