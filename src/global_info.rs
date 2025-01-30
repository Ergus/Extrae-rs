#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::atomic;

use crate::{Merger,buffer,global_config::GlobalConfig};
use crate::perf::SomeEvent;

pub struct GlobalInfo {

    buffer_set: crate::bufferset::BufferSet,
    name_set: crate::nameset::NameSet,

    threads_running: atomic::AtomicU32,

    pub(crate) config: GlobalConfig,

    pub thread_event_id: u16,

    // Hardware events ID
    pub events_info: Vec<(String, u16)>,
}

impl GlobalInfo {
    /// Global info constructor.
    ///
    /// This is a critical function that we use to initialize the
    /// profiler information. It initializes internal variables,
    /// hwcounters and all global information that is intended to not
    /// change during the execution and need to be shared/accesed by
    /// all the threads when created.
    ///
    /// That constant information can be accessed lock free as they
    /// are not mutable.
    fn new() -> Self
    {
        println!("Initializing profiler");

        let config = GlobalConfig::new();

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

        // Register all the possible supported events to preserve the ids.
        let all_events_info: BTreeMap<&str, u16> =
            SomeEvent::EVENTS_LIST
                .iter()
                .map(|(name, _)| {
                    let eid  = name_set.register_event_name_internal(&name);
                    (*name, eid)
                })
                .collect();


        // For events info we take all the input names and then
        // iterate filtering only the valid names and registering
        // them.
        let events_info: Vec<(String, u16)> =
            config
                .counters
                .iter()
                .filter(|name| ! matches!(SomeEvent::event_from_str(name), SomeEvent::None))
                .map(|name| {
                    let eid  = all_events_info.get(name.as_str()).unwrap();
                    (name.clone(), *eid)
                })
                .collect();

        println!("Profiler enabled counters: {:?}", events_info);

        Self {
            buffer_set,
            name_set,
            threads_running: atomic::AtomicU32::new(0),
            thread_event_id,
            config,
            events_info
        }
    }

    /// Create a new buffer for a thread
    ///
    /// This function is called every time a new thread is created and
    /// it updates the running_threads counter.
    fn init_buffer(&mut self, tid: std::thread::ThreadId, name: &str) -> buffer::Buffer
    {
        self.threads_running.fetch_add(1, atomic::Ordering::Relaxed);
        self.buffer_set.get_buffer(tid, name)
    }

    /// Save a buffer information before a thread is destroyed
    ///
    /// This function is called every time a new thread finalizes and
    /// it updates the running_threads counter. When the counter
    /// reaches zero it calls the finalize function to perform io
    /// actions.
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

    /// This is the finalization function.
    ///
    /// It performs the io operations to output the .row, .pcf and
    /// merge the .prv file (when automerge is enabled)
    ///
    /// This is not in the Drop because this is called by the last
    /// thread when destroyed. Rust seems not to call the destructor
    /// for global variables even at the end of the main, but it calls
    /// the thread_local variables destructor.
    ///
    /// For this reason this class has the threads_running counter
    /// that is updated every time a new thread is created/destroyed
    /// (in the finalize_buffer function). When it reaches zero it
    /// calls this function. As this can only happen when the last
    /// thread is destroyed we don't have race condition risk.
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

        if self.config.automerge {
            Merger::new(output_path)      // path to read from
                .create_prv(output_path)  // path to write to
                .expect("Error creating PRV file");
        }

        println!("# Profiler TraceDir: {}", output_path.to_str().unwrap());
    }

}

/// This is the variable to store the global information.
/// Here I will use an option in order to allow lazy initialization because
/// I need it to be mutable.
///
/// This forces me to use unsafe wrappers to access it, but at leats I
/// won't need to have a global mutex for every access to the global
/// information.
///
/// The internal functions already have a lock when needed.  The
/// initialization is intended to take place in the main, and the risk
/// of multiple attempts to initialize is very low, so we ignore it
/// for now.
static mut INFO: Option<GlobalInfo> = None;

impl GlobalInfo {

    /// Get a shared const reference to the Global info.
    /// This is for internal use and our code ensures to use it
    /// properly to access only constant values.
    /// For mutable values I provide proper access wrappers and the
    /// inner functions already have a lock when needed..
    pub(crate) fn as_ref() -> &'static GlobalInfo
    {
        unsafe {
            if let None = INFO {
                INFO = Some(GlobalInfo::new());
            }

            INFO.as_ref().unwrap_unchecked()
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

    /// Internal api function to register a new event name.
    /// The arguments are as described ny their names.
    /// Remember that the events are identified by their id, not by
    /// their names; so, multiple ids can repeat names and they will be
    /// difficult to identify in the final trace.
    #[inline]
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

    /// The paraver format can assign names also to the values of the
    /// events. Even when not needed, this is a useful feature to use.
    pub fn register_event_value_name(
        event_name: &str,
        file_name: Option<&str>,
        line: Option<u32>,
        event: u16,
        value: Option<u32>
    ) -> u32 {
        unsafe {
            INFO.get_or_insert_with(|| GlobalInfo::new())
                .name_set
                .register_event_value_name(event_name, file_name, line, event, value)
        }
    }

    /// Get the event value associated information.
    /// This function was intended to be used in the subscriber, but
    /// I preferred to use a different approach to avoid creating excessive
    /// contention.
    /// This function takes a read lock internally while it searches in a map.
    pub(crate) fn get_event_value_info(
        event: u16,
        value: Option<u32>
    ) -> Option<crate::nameset::NameInfo> {
        unsafe {
            INFO.get_or_insert_with(|| GlobalInfo::new())
                .name_set
                .get_event_value_info(event, value)
        }
    }


}


