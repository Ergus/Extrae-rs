#![allow(dead_code)]

use std::sync::{LazyLock,RwLock};

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

        // crate::thread_info::ThreadInfo::emplace_event(
        //     thread_event_id, 1);


        Self {
            buffer_set,
            name_set,
            thread_event_id
        }
    }

    fn finalize(&self) {

        assert_eq!(self.buffer_set.thread_running, 0);

        println!("Executing global destructor");

        // crate::thread_info::ThreadInfo::emplace_event(
        //     self.thread_event_id, 0);

        let output_path = self.buffer_set.trace_directory_path.as_path();

	self.buffer_set
            .create_row(output_path)
            .expect("Error creatiiing ROW file");
        self.name_set
            .create_pcf(output_path)
            .expect("Error creating PCF file");

	//std::cout << "# Profiler TraceDir: " << traceDirectory << std::endl;


    }

}

//static mut INFO: Option<GlobalInfo> = None;

static INFO: LazyLock<RwLock<GlobalInfo>>
 = LazyLock::new(|| { RwLock::new(GlobalInfo::new())});

impl GlobalInfo {

    // This requires mutable access to the variable.
    pub(crate) fn get_buffer(tid: std::thread::ThreadId) -> crate::buffer::Buffer
    {
        INFO.write().unwrap().buffer_set.get_buffer(tid)
    }

    // This requires mutable access to the variable.
    pub(crate) fn save_buffer(mut buffer: crate::buffer::Buffer)
    {
        buffer.flush().expect("Failed to flush buffer to file");
        if INFO.write().unwrap().buffer_set.save_buffer(buffer) == 0 {
            INFO.read().unwrap().finalize();
        }
    }

    pub fn register_event_name(
        event_name: &str,
        file_name: &str,
        line: u32,
        event: u16
    ) {
        INFO.write().unwrap()
            .name_set
            .register_event_name(event_name, file_name, line, event);
    }
}










