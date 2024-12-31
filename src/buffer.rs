#![allow(dead_code)]

use std::{io::{Read, Seek, Write}, os::unix::fs::FileExt};

/// Get nano-seconds since the trace begins for a given timePoint
fn get_nano_seconds() -> u128 {
    static START_TIMESTAMP: std::sync::OnceLock::<std::time::Instant> = std::sync::OnceLock::<std::time::Instant>::new();
    START_TIMESTAMP.get_or_init(std::time::Instant::now).elapsed().as_nanos()
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct TraceHeader {
    id: u32,
    tid: std::thread::ThreadId,
    start_gtime: u64,
    total_flushed: u32,
}

impl TraceHeader {

    fn new(id: u32, tid: &std::thread::ThreadId, start_gtime: &std::time::Duration) -> Self
    {
        Self {
            id,
            tid: *tid,
            start_gtime: start_gtime.as_secs() ,
            total_flushed: 0
        }
    }

    fn as_bytes(&self) ->  &[u8]
    {
        unsafe {
            std::slice::from_raw_parts(
                self as *const TraceHeader as *const u8,
                std::mem::size_of::<TraceHeader>()
            )
        }
    }
}
impl std::fmt::Display for TraceHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "id:{} tid:{:?} start_gtime:{} total_flushed:{}",
            self.id, self.tid, self.start_gtime, self.total_flushed)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct EventEntry {
    time: u64,
    id: u16,
    core: u16,
    value: u32,
}

impl EventEntry {
    fn new(id: u16, value: u32) -> Self
    {
        Self {
            time: u64::try_from(get_nano_seconds())
                .expect("Time conversion overflow"),
            id,
            core: u16::try_from(nix::sched::sched_getcpu()
                .expect("Could not get cpuID"))
                .expect("cpuid conversion overflow"),
            value,
        }
    }

    const fn bytes() -> usize
    {
        std::mem::size_of::<Self>()
    }

    fn as_line(&self, thread: u32) -> String
    {
        // 2:cpu:appl:task:thread:time:event:value
        format!("2:{}:{}:{}:{}:{}:{}:{}",
            self.core, 1, 1, thread, self.time, self.id, self.value)
    }
}

impl std::fmt::Display for EventEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "core:{} time:{} id:{} value:{}",
            self.core, self.time, self.id, self.value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BufferInfo {
    header: TraceHeader,
    entries: Vec<EventEntry>,
}

impl BufferInfo {
    const MAX_ENTRIES: usize = (1024 * 1024 + EventEntry::bytes() - 1) / EventEntry::bytes();

    fn new(
        id: u32,
        tid: &std::thread::ThreadId,
        start_gtime: &std::time::Duration
    ) -> Self {
        Self {
            header: TraceHeader::new(id, &tid, &start_gtime),
            entries: Vec::<EventEntry>::with_capacity(Self::MAX_ENTRIES)
        }
    }

    pub fn from_file(file: &mut std::fs::File) -> Self
    {
        let mut buf_reader = std::io::BufReader::new(file);

        const HDRSIZE: usize = std::mem::size_of::<TraceHeader>();
        const EVTSIZE: usize = std::mem::size_of::<EventEntry>();

        // Allocate a buffer to read the structs
        let mut tmp = vec![0u8; HDRSIZE];
        buf_reader.read_exact(&mut tmp).expect("Error reading header from file");

        let header: TraceHeader = unsafe {
            *(tmp.as_ptr() as *const TraceHeader) 
        };

        // Now create a vector for the number of entries in the header
        // and import the events "IN PLACE". The unsafe code is to
        // avoid a temporal buffer and coping.
        let n_entries: usize = header.total_flushed as usize;
        let mut entries = Vec::<EventEntry>::with_capacity(n_entries);

        buf_reader.read_exact(
            unsafe {
                std::slice::from_raw_parts_mut(
                    entries.as_mut_ptr() as *mut u8,
                    n_entries * EVTSIZE
                )
            }
        ).expect("Error reading events from file");

        unsafe {
            entries.set_len(n_entries);
        }

        Self { header, entries }

    }


    fn entries_as_bytes(&self) -> &[u8]
    {
        unsafe {
            // Convert the slice to a slice of bytes
            std::slice::from_raw_parts(
                self.entries.as_ptr() as *const u8,
                self.entries.len() * std::mem::size_of::<EventEntry>(),
            )
        }
    }

    fn flush_to_file(&mut self, file: &mut std::fs::File) -> std::io::Result<()>
    {
        if self.entries.is_empty() {
            return Ok(());
        }

        let n_entries: usize = self.entries.len();

        self.header.total_flushed += n_entries as u32;

        file.write_at(self.header.as_bytes(), 0)?;
        file.seek(std::io::SeekFrom::End(0))?;
        file.write_all(self.entries_as_bytes())?;
        file.flush()?;

        self.entries.clear();

        Ok(())
    }

    fn emplace_event(&mut self, id: u16, value: u32)
    {
        self.entries.push(EventEntry::new(id, value));
    }

    fn is_full(&self) -> bool
    {
        assert!(self.entries.len() <= BufferInfo::MAX_ENTRIES);
        self.entries.len() == BufferInfo::MAX_ENTRIES
    }

    fn is_empty(&self) -> bool
    {
        self.entries.is_empty()
    }
}

impl std::fmt::Display for BufferInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {

        writeln!(f, "{}", self.header)?;

        for entry in &self.entries {
            writeln!(f, "{}", entry)?;
        }

        write!(f, "\n")
    }
}

pub struct Buffer {
    path: std::path::PathBuf,
    file: Option<std::fs::File>,
    info: BufferInfo,
}

impl Buffer {

    pub fn new(
        id: u32,
        tid: &std::thread::ThreadId,
        path: std::path::PathBuf,
        start_gtime: &std::time::Duration
    ) -> Self {
        Self {
            path,
            file: None,
            info: BufferInfo::new(id, &tid, &start_gtime)
        }
    }

    pub fn id(&self) -> u32
    {
        self.info.header.id
    }

    pub fn tid(&self) -> std::thread::ThreadId
    {
        self.info.header.tid
    }


    fn from_file(path: std::path::PathBuf) -> Self
    {
        let mut file = std::fs::File::open(&path).unwrap();

        let info = BufferInfo::from_file(&mut file);

        Self { path, file: None, info }
    }


    pub(crate) fn flush(&mut self) -> std::io::Result<()>
    {
        if self.info.is_empty() {
            return Ok(());
        }

        // We open the file the first time we need to flush the data.
        // I do this because some threads may not create traces, so no
        // file creation is needed.
        if self.file.is_none() {
            self.file = Some(
                std::fs::OpenOptions::new()
                    .write(true)
                    .create(true) // Creates the file if it does not exist
                    .open(&self.path).unwrap()
            );
        }

        self.info.flush_to_file(self.file.as_mut().unwrap())
    }

    pub fn emplace_event(&mut self, id: u16, value: u32)
    {
        self.info.emplace_event(id, value);
    }
}


impl Drop for Buffer {
    fn drop(&mut self) {
        self.flush().expect("Failed to flush buffer to file on drop");
    }
}


#[cfg(test)]
mod profiler{

    use std::str::FromStr;

    use super::*;

    #[test]
    fn bufferinfo_construct()
    {
        let mut info = BufferInfo::new(
            1,
            &std::thread::current().id(),
            &std::time::Duration::default()
        );

        info.emplace_event(1, 1);
        info.emplace_event(1, 2);
        info.emplace_event(2, 1);
        info.emplace_event(2, 2);
    }

    #[test]
    fn bufferinfo_serialize()
    {
        let path = std::path::PathBuf::from_str("/tmp/bufferinfo_serialize").unwrap();

        // Create a buffer with 6 entries
        let mut info = BufferInfo::new(
            1,
            &std::thread::current().id(),
            &std::time::Duration::default()
        );

        info.emplace_event(1, 1);
        info.emplace_event(2, 7);
        info.emplace_event(3, 8);
        info.emplace_event(4, 9);
        info.emplace_event(5, 10);
        info.emplace_event(6, 11);

        let mut cloned_info = info.clone();
        cloned_info.header.total_flushed = 6;

        let mut file = std::fs::File::create_new(&path).expect("Error creating file");
        info.flush_to_file(&mut file).expect("Failed to flush");

        assert!(path.exists());

        let mut file = std::fs::File::open(&path).unwrap();
        let imported_info = BufferInfo::from_file(&mut file);

        std::fs::remove_file(path).unwrap();

        assert_eq!(cloned_info, imported_info);
    }

    #[test]
    fn buffer_construct_destruct()
    {
        let path = std::path::PathBuf::from_str("/tmp/buffer_construct_destruct").unwrap();

        let mut buff = Buffer::new(
            1,
            &std::thread::current().id(),
            path.clone(),
            &std::time::Duration::default()
        );

        buff.emplace_event(1, 1);

        // Assert that the file is created
        drop(buff);
        assert!(path.exists());
        std::fs::remove_file(path).unwrap();
    }


    #[test]
    fn buffer_construct_destruct_empty()
    {
        let path = std::path::PathBuf::from_str("/tmp/buffer_construct_destruct_empty").unwrap();

        let buff = Buffer::new(
            1,
            &std::thread::current().id(),
            path.clone(),
            &std::time::Duration::default()
        );

        // Assert that the file is NOT created
        drop(buff);
        assert!(!path.exists());
    }


    #[test]
    fn buffer_serialize()
    {
        let path = std::path::PathBuf::from_str("/tmp/buffer_serialize").unwrap();

        // Create a buffer with 6 entries
        let mut buff = Buffer::new(
            1,
            &std::thread::current().id(),
            path.clone(),
            &std::time::Duration::default()
        );

        buff.emplace_event(1, 1);
        buff.emplace_event(2, 7);
        buff.emplace_event(3, 8);
        buff.emplace_event(4, 9);
        buff.emplace_event(5, 10);
        buff.emplace_event(6, 11);

        // Make a clone of the info to compare.
        let mut cloned_info = buff.info.clone();

        // Trick to match, the total_flushed contains the counter of
        // the events that are not in the buffer anymore.
        cloned_info.header.total_flushed = 6;

        // Flush the buffer.
        buff.flush().unwrap();

        assert!(path.exists());

        let mut file = std::fs::File::open(&path).unwrap();
        let imported_info = BufferInfo::from_file(&mut file);

        std::fs::remove_file(path).unwrap();

        assert_eq!(cloned_info, imported_info);

    }

    #[test]
    fn buffer_serialize_multi()
    {
        let path = std::path::PathBuf::from_str("/tmp/buffer_serialize_multi").unwrap();

        { // Create a buffer with 6 entries in 3 steps
            let mut buff = Buffer::new(
                1,
                &std::thread::current().id(),
                path.clone(),
                &std::time::Duration::default()
            );

            buff.emplace_event(0, 1);
            buff.emplace_event(1, 2);
            buff.flush().unwrap();

            buff.emplace_event(2, 3);
            buff.emplace_event(3, 4);
            buff.flush().unwrap();

            buff.emplace_event(4, 5);
            buff.emplace_event(5, 6);
        }

        assert!(path.exists());

        let mut file = std::fs::File::open(&path).unwrap();
        let imported_info = BufferInfo::from_file(&mut file);

        assert_eq!(imported_info.header.total_flushed, 6);
        for i in 0..6 {
            assert_eq!(imported_info.entries[i].id, i as u16);
            assert_eq!(imported_info.entries[i].value, (i + 1) as u32);
        }

        std::fs::remove_file(path).unwrap();
    }
}
