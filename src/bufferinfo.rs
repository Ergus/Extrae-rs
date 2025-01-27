#![allow(dead_code)]

use crate::event;
use std::{io::{Read, Seek, Write}, os::unix::fs::FileExt};

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

#[derive(Debug, Clone, PartialEq)]
pub struct BufferInfo {
    pub(crate) header: TraceHeader,
    pub(crate) entries: Vec<event::EventEntry>,
}

impl BufferInfo {
    const MAX_ENTRIES: usize = (1024 * 1024 + std::mem::size_of::<event::EventEntry>() - 1) / std::mem::size_of::<event::EventEntry>();

    pub(crate) fn new(
        id: u32,
        tid: &std::thread::ThreadId,
        start_gtime: &std::time::Duration
    ) -> Self {
        Self {
            header: TraceHeader::new(id, &tid, &start_gtime),
            entries: Vec::<event::EventEntry>::with_capacity(Self::MAX_ENTRIES)
        }
    }

    pub fn from_file(file: &mut std::fs::File) -> Self
    {
        let mut buf_reader = std::io::BufReader::new(file);

        const HDRSIZE: usize = std::mem::size_of::<TraceHeader>();
        const EVTSIZE: usize = std::mem::size_of::<event::EventEntry>();

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
        let mut entries = Vec::<event::EventEntry>::with_capacity(n_entries);

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
                self.entries.len() * std::mem::size_of::<event::EventEntry>(),
            )
        }
    }

    fn entries_to_file(
        &mut self,
        file: &mut std::fs::File
    ) -> std::io::Result<()> {

        file.seek(std::io::SeekFrom::End(0))?;
        let mut writer = std::io::BufWriter::new(file);

        writer.write_all(self.entries_as_bytes())?;
        writer.flush()?;

        self.entries.clear();

        Ok(())
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

        self.entries_to_file(file)?;

        Ok(())
    }

    pub(crate) fn emplace_event(&mut self, id: u16, value: u32)
    {
        self.entries.push(event::EventEntry::new(id, value));
    }

    pub(crate) fn emplace_events(&mut self, entries: &[(u16, u32)])
    {
        let hdr = crate::event::EventHeader::new();

        for &entry in entries.iter() {
            self.entries.push(
                event::EventEntry { hdr: hdr.clone(), info: entry.into() }
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

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, event::EventEntry>
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
    type Output = event::EventEntry;

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
        let info = BufferInfo::new(
            1,
            &std::thread::current().id(),
            &std::time::Duration::default()
        );

        assert_eq!(info.header.total_flushed, 0);
        assert_eq!(info.header.id, 1);
        assert!(info.is_empty());
    }

    #[test]
    fn bufferinfo_emplace_event()
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

        assert_eq!(info[0].info, (1, 1).into());
        assert_eq!(info[1].info, (1, 2).into());
        assert_eq!(info[2].info, (2, 1).into());
        assert_eq!(info[3].info, (2, 2).into());
    }


    #[test]
    fn bufferinfo_emplace_events()
    {
        let mut info = BufferInfo::new(
            1,
            &std::thread::current().id(),
            &std::time::Duration::default()
        );
        assert!(info.is_empty());

        info.emplace_events(
            &[(1, 1), (1, 2), (2, 1), (2, 2)]
        );

        assert!(!info.is_empty());
        assert_eq!(info[0].info, (1, 1).into());
        assert_eq!(info[1].info, (1, 2).into());
        assert_eq!(info[2].info, (2, 1).into());
        assert_eq!(info[3].info, (2, 2).into());

        assert_eq!(info[0].hdr, info[1].hdr);
        assert_eq!(info[0].hdr, info[2].hdr);
        assert_eq!(info[0].hdr, info[3].hdr);

        // Check that 3 entries does not full the buffer
        assert!(!info.is_full());
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

        assert!(!imported_info.is_empty());
        assert!(!imported_info.is_full());

        // Tests imported values
        assert_eq!(imported_info[0].info, (1, 1).into());
        assert_eq!(imported_info[1].info, (2, 7).into());
        assert_eq!(imported_info[2].info, (3, 8).into());
        assert_eq!(imported_info[3].info, (4, 9).into());
        assert_eq!(imported_info[4].info, (5, 10).into());
        assert_eq!(imported_info[5].info, (6, 11).into());

        assert_eq!(cloned_info, imported_info);
    }
}
