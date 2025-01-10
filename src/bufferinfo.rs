#![allow(dead_code)]

use std::{io::{Read, Seek, Write}, os::unix::fs::FileExt};

/// Get nano-seconds since the trace begins for a given timePoint
fn get_nano_seconds() -> u128 {
    static START_TIMESTAMP: std::sync::OnceLock::<std::time::Instant> = std::sync::OnceLock::<std::time::Instant>::new();
    START_TIMESTAMP.get_or_init(std::time::Instant::now).elapsed().as_nanos()
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TraceHeader {
    pub(crate) id: u32,
    pub(crate) tid: std::thread::ThreadId,
    pub(crate) start_gtime: u64,
    pub(crate) total_flushed: u32,
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
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub(crate) struct EventHeader {
    pub(crate) time: u64,
    pub(crate) core: u16,
}

impl EventHeader {
    fn new() -> Self
    {
        Self {
            time: u64::try_from(get_nano_seconds())
                .expect("Time conversion overflow"),
            core: u16::try_from(nix::sched::sched_getcpu()
                .expect("Could not get cpuID"))
                .expect("cpuid conversion overflow"),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub(crate) struct EventInfo {
    pub(crate) id: u16,
    pub(crate) value: u32,
}

// Automatic convert tuple to EventInfo
impl std::convert::From<(u16, u32)> for EventInfo {
    fn from(tuple: (u16, u32)) -> Self {
        Self {
            id: tuple.0,
            value: tuple.1,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct EventEntry {
    pub(crate) hdr: EventHeader,
    pub(crate) info: EventInfo,
}

impl EventEntry {
    fn new(id: u16, value: u32) -> Self
    {
        Self {
            hdr: EventHeader::new(),
            info: EventInfo { id, value }
        }
    }
}

// Needed to sort in the heap
impl PartialOrd for EventEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EventEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hdr.time.cmp(&other.hdr.time)
    }
}

impl std::fmt::Display for EventEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "core:{} time:{} id:{} value:{}",
            self.hdr.core, self.hdr.time, self.info.id, self.info.value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BufferInfo {
    pub(crate) header: TraceHeader,
    pub(crate) entries: Vec<EventEntry>,
}

impl BufferInfo {
    const MAX_ENTRIES: usize = (1024 * 1024 + std::mem::size_of::<EventEntry>() - 1) / std::mem::size_of::<EventEntry>();

    pub(crate) fn new(
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

    pub(crate) fn flush_to_file(
        &mut self,
        file: &mut std::fs::File
    ) -> std::io::Result<()> {
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

    pub(crate) fn emplace_event(&mut self, id: u16, value: u32)
    {
        self.entries.push(EventEntry::new(id, value));
    }

    pub(crate) fn emplace_events(&mut self, entries: &[(u16, u32)])
    {
        let hdr = EventHeader::new();

        for &entry in entries.iter() {
            self.entries.push(
                EventEntry { hdr: hdr.clone(), info: entry.into() }
            );
        }
    }

    pub(crate) fn is_full(&self) -> bool
    {
        assert!(self.entries.len() <= BufferInfo::MAX_ENTRIES);
        self.entries.len() == BufferInfo::MAX_ENTRIES
    }

    pub(crate) fn is_empty(&self) -> bool
    {
        self.entries.is_empty()
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, EventEntry>
    {
        self.entries.iter()
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

impl std::ops::Index<usize> for BufferInfo {
    type Output = EventEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

#[cfg(test)]
mod profiler {

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
}
