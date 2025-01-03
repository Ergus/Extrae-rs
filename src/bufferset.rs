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
    threadid_map: Arc<RwLock<HashMap<ThreadId, u32>>>,
    threads_counter: atomic::AtomicU32,
    threads_running: atomic::AtomicU32,

    pub(crate) start_system_time: std::time::Duration,
    pub(crate) trace_directory_path: std::path::PathBuf,
}

impl BufferSet {

    pub fn new(
        start_system_time: std::time::Duration,
        trace_directory_path: std::path::PathBuf
    ) -> Self {
        Self {
            threadid_map: Arc::new(RwLock::new(HashMap::new())),
            threads_counter: atomic::AtomicU32::new(0),
            threads_running: atomic::AtomicU32::new(0),
            start_system_time,
            trace_directory_path
        }
    }

    /// Create a Buffer_t associated with a thread id hash
    ///
    /// The threadIds sometimes are reused after a thread is destroyed.
    ///
    /// We keep the associative map <tid, id> in order to reuse Buffer
    /// ids.
    ///
    /// The extra cost for this is that we need to take a read lock
    /// once (on thread construction or when emitting the first event
    /// from a thread) in order to get it's associated buffer.  This
    /// function is responsible to take the lock and return the
    /// associated buffer id.  When a threadId is seen for a first
    /// time this function creates the new id in the map, but won't
    /// modifies the map, only the atomic threads counter is
    /// increased.
    /// The map only adds new values on thread destruction to "remember"
    /// in the future if it sees the same thread id again.
    pub fn get_buffer(&mut self, tid: std::thread::ThreadId) -> buffer::Buffer
    {
        // We attempt to take the read lock only to check if the id
        // exists and release it immediately.  The thread counted
        // needs to be atomic because it is modified with the read
        // lock taken (not the write)
        let id: u32 = {
            match self.threadid_map
                .read()
                .expect("Failed to get threadid_map read lock")
                .get(&tid) {
                    Some(&value) => value,
                    None => self.threads_counter.fetch_add(1, atomic::Ordering::Relaxed) + 1,
                }
        };

        self.threads_running.fetch_add(1, atomic::Ordering::Relaxed);

        buffer::Buffer::new(
            id,
            &tid,
            self.trace_directory_path.join(format!("Trace_{}.bin", id)),
            &self.start_system_time
        )
    }

    /// When a thread is destroyed it's buffer id is saved back to the
    /// buffer set in order to avoid creation and destructions too
    /// often.
    /// If the thread id was already used, then this function basically
    /// does nothing, but confirm that the tid and the id it contains
    /// are the same of the incoming buffer.
    /// This function takes the write lock as the most frequent action
    /// is to register new ids.
    pub fn save_buffer_id(&mut self, buffer: &buffer::Buffer) -> u32
    {
        match self.threadid_map
            .write()
            .expect("Failed to get threadid_map write lock")
            .entry(buffer.tid()) {
                Entry::Occupied(entry) => {
                    assert_eq!(entry.get(), &buffer.id());
                },
                Entry::Vacant(entry) => {
                    entry.insert(buffer.id());
                    self.threads_running.fetch_sub(1, atomic::Ordering::Relaxed);
                }
            };

        // The thread counter is in the worst possible place (here)
        // But that's because this is the safest possible place.
        self.threads_running.load(atomic::Ordering::Relaxed)
    }

    /// Write the trace.row file on exit.
    pub fn create_row(&self, trace_dir: &std::path::Path) -> std::io::Result<()>
    {
        // Yes I am a bit paranoic.
        assert_eq!(self.threads_running.load(atomic::Ordering::Relaxed), 0);

        let hostname = nix::unistd::gethostname()
            .expect("Error getting hostname")
            .into_string().expect("Failed to convert hostname to string");

        // Get the total number of cores in the system
        let ncores = {
            match nix::unistd::sysconf(
                nix::unistd::SysconfVar::_NPROCESSORS_CONF
            ) {
                Ok(Some(value)) => value,
                _ => panic!("Error getting the system number of cores"),
            }
        };

        let nthreads = self.threads_counter.load(atomic::Ordering::Relaxed);

        // Lets be paranoic
        assert_eq!(
            self.threadid_map.read().expect("Error getting threadid_map read lock").len(),
            nthreads as usize,
            "The number of thread ids does not match with the total stored in the threadid_map"
        );

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
