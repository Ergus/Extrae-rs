#![allow(dead_code)]

use std::io::Write;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::thread::ThreadId;
use std::sync::atomic;

use crate::buffer;

/// BufferSet container
/// 
/// This is container stores the buffer for every thread. in a map
/// <tid, Buffer> This is intended to remember the tid to reuse the
/// Buffer because the tid is usually recycled after a thread is
/// deleted.  This class is allocated inside a shared_ptr to enforce
/// that it will be deleted only after the destruction of all the
/// threads.  The Global container holds a reference to it; but every
/// ThreadInfo will also keep one reference.
/// 
/// This is because it seems like on GNU/Linux the global variables
/// are destructed after the main thread; but in MSWindows the Global
/// variables seems to be removed before the main thread completes.
pub struct BufferSet {
    events_map: Arc<RwLock<HashMap<ThreadId, buffer::Buffer>>>,
    thread_counter: atomic::AtomicU32,
    thread_running: atomic::AtomicU32,

    pub(crate) start_system_time: std::time::Duration,
    pub(crate) trace_directory_path: std::path::PathBuf,
}

impl BufferSet {

    pub fn new(
        start_system_time: std::time::Duration,
        trace_directory_path: std::path::PathBuf
    ) -> Self {
        Self {
            events_map: Arc::new(RwLock::new(HashMap::new())),
            thread_counter: atomic::AtomicU32::new(0),
            thread_running: atomic::AtomicU32::new(0),
            start_system_time,
            trace_directory_path
        }
    }

    /// Get the Buffer_t associated with a thread id hash
    ///
    /// The threadIds are usually reused after a thread is destroyed.
    /// Opening/closing files on every thread creation/deletion may be
    /// too expensive; especially if the threads are created destroyed
    /// very frequently.
    ///
    /// We keep the associative map <tid, Buffer> in order to reuse
    /// Buffer and only execute IO operations when the buffer is full
    /// or at the end of the execution.
    ///
    /// The extra cost for this is that we need to take a lock once
    /// (on thread construction or when emitting the first event from
    /// a thread) in order to get it's associated buffer.  This
    /// function is responsible to take the lock and return the
    /// associated buffer.  When a threadId is seen for a first time
    /// this function creates the new entry in the map, construct the
    /// Buffer and assign an ordinal id for it.  Any optimization here
    /// will be very welcome.
    pub fn get_buffer(&mut self, tid: std::thread::ThreadId) -> buffer::Buffer
    {
        // We attempt to take the read lock first. If this tid was
        // already used then we need to take the write lock
        // temporarily to extract it.
        // Otherwise no look is needed at all and we can create a new
        // buffer lock-free
        let contains =
            self.events_map
                .read()
                .expect("Failed to get events_map read lock")
                .contains_key(&tid);

        if contains {
            // Get write lock and extract with the least possible
            // contention,
            self.events_map
                .write()
                .expect("Failed to get events_map read lock")
                .remove(&tid)
                .unwrap()

        } else {

            let counter = self.thread_counter.fetch_add(1, atomic::Ordering::Relaxed);
            self.thread_running.fetch_add(1, atomic::Ordering::Relaxed);

            let filename = self
                .trace_directory_path
                .join(format!("Trace_{}", counter + 1));

            buffer::Buffer::new(
                counter + 1,
                &tid,
                filename,
                &self.start_system_time
            )
        }
    }

    /// When a thread is destroyed it's buffer is saved back to the
    /// buffer set in order to avoid creation and destruction too
    /// often.
    pub fn save_buffer(&mut self, buffer: buffer::Buffer) -> u32
    {
        match self.events_map
            .write()
            .expect("Failed to get events_map lock")
            .entry(buffer.tid()) {
                Entry::Occupied(_) => panic!("Error reinserting buffer for existing tid"),
                Entry::Vacant(entry) => {
                    entry.insert(buffer);
                    self.thread_running.fetch_sub(1, atomic::Ordering::Relaxed);
                }
            };

        self.thread_running.load(atomic::Ordering::Relaxed)
    }

    /// Write the trace.row file on exit.
    pub fn create_row(&self, trace_dir: &std::path::Path) -> std::io::Result<()>
    {
        let hostname = nix::unistd::gethostname()
            .expect("Error getting hostname")
            .into_string().expect("Failed to convert hostname to string");

        let ncores = {
            match nix::unistd::sysconf(
                nix::unistd::SysconfVar::_NPROCESSORS_CONF
            ) {
                Ok(Some(value)) => value,
                _ => panic!("Error getting the number of cores"),
            }
        };

        let nthreads = self.thread_counter.load(atomic::Ordering::Relaxed);

        let rowfile = std::fs::File::create(trace_dir.join("Trace.row")).unwrap();
        let mut writer = std::io::BufWriter::new(rowfile);

        writeln!(writer, "LEVEL CPU SIZE {}", ncores)?;
        for i in 1..=ncores {
            writeln!(writer, "{}.{}", i, hostname)?;
        }

        writeln!(writer, "\nLEVEL NODE SIZE 1")?;
        writeln!(writer, "{}", hostname)?;

        writeln!(writer, "\nLEVEL THREAD SIZE {}", nthreads)?;

        for i in 1..=nthreads {
            writeln!(writer, "THREAD 1.1.{}", i)?;
        }

        Ok(())
    }
}
