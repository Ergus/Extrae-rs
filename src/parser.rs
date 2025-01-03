#![allow(dead_code)]

use std::io::{Read, Write};
use std::fs::File;
use std::iter::Iterator;

use crate::buffer;

// Iterator for the array inside the file.
struct TraceIterator {
    pub(crate) header: buffer::TraceHeader,
    buf_reader: std::io::BufReader<File>,
    remaining: usize,
}

impl TraceIterator {
    fn open(path: &std::path::Path) -> Self {
        let file = File::open(path).expect("Error opening file");
        let mut buf_reader = std::io::BufReader::new(file);

        const HDRSIZE: usize = std::mem::size_of::<buffer::TraceHeader>();

        // Allocate a buffer to read the structs
        let mut tmp = vec![0u8; HDRSIZE];
        buf_reader.read_exact(&mut tmp).expect("Error reading header from file");

        let header: buffer::TraceHeader = unsafe {
            *(tmp.as_ptr() as *const buffer::TraceHeader) 
        };

        let remaining = header.total_flushed as usize;

        Self {header, buf_reader, remaining}
    }
}

impl Iterator for TraceIterator {
    type Item = buffer::EventEntry;

    fn next(&mut self) -> Option<Self::Item>
    {
        if self.remaining == 0 {
            return None;
        }

        let mut entry = buffer::EventEntry::default();

        match self.buf_reader.read_exact(
            unsafe {
                std::slice::from_raw_parts_mut(
                    &mut entry as *mut _ as *mut u8,
                    std::mem::size_of::<buffer::EventEntry>()
                )
            }
        ) {
            Ok(_) => Some(entry),
            Err(_) => None
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ExtendedEvent {
    time: u64,
    tid: u32,
    core: u16,
    events: Vec<buffer::EventInfo>
}

impl ExtendedEvent {
    fn new(id: u32, event: &buffer::EventEntry) -> Self
    {
        Self {
            time: event.hdr.time,
            tid: id,
            core: event.hdr.core,
            events: vec![event.info]

        }
    }
}

impl std::fmt::Display for ExtendedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "2:{}:{}:{}:{}:{}", self.core, 1, 1, self.tid, self.time)?;
        for &event in self.events.iter() {
            write!(f, ":{}:{}", event.id, event.value)?;
        }
        write!(f, "")
    }
}

struct Merger
{
    dir_path: std::path::PathBuf,
    file_paths: Vec<std::path::PathBuf>
}

impl Merger {

    /// Get a vector of paths for all the files with a given extension.
    fn get_files_with_extension(
        dir: &std::path::Path,
        extension: &str
    ) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(dir)
            .expect("Failed to read directory")
            .filter_map(
                |entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    if path.is_file()
                        && path.extension()
                            .is_some_and(|fext| extension == fext) {
                                return Some(path)
                            }
                    None
                }
            )
            .collect()
    }

    fn new(dir: &std::path::Path) -> Self
    {
        Self {
            dir_path: std::path::PathBuf::from(dir),
            file_paths: Merger::get_files_with_extension(dir, "bin"),
        }
    }

    fn merge_files(&self)
    {
        let file = std::fs::File::create(self.dir_path.join("Trace.prv")).unwrap();
        let mut writer = std::io::BufWriter::new(file);

        let mut heap = std::collections::BinaryHeap::new();

        let mut trace_iters: Vec<_>
            = self.file_paths
                .iter()
                .map(|path| TraceIterator::open(path.as_path()))
                .collect();

        let total_events: u32 = trace_iters.iter().map(|item| item.header.total_flushed).sum();

        let all_equal = trace_iters.windows(2).all(|pair| pair[0].header.start_gtime == pair[1].header.start_gtime);
        assert!(all_equal, "Some global time differs in trace headers");

        let mut counter = 0;

        for (index, trace_iter) in trace_iters.iter_mut().enumerate() {
            if let Some(entry) = trace_iter.next() {
                heap.push(std::cmp::Reverse((entry, index)));
                counter += 1;
            }
        }

        while let Some(std::cmp::Reverse((entry, index))) = heap.pop() {

            let mut ext_entry = ExtendedEvent::new(trace_iters[index].header.id, &entry);

            while let Some(next_entry) = trace_iters[index].next() {
                counter += 1;

                if next_entry.hdr == entry.hdr {
                    ext_entry.events.push(next_entry.info);
                } else {
                    heap.push(std::cmp::Reverse((next_entry, index)));
                }
            }

            writeln!(writer,"{}", ext_entry).unwrap();
        }

        assert_eq!(total_events, counter);
    }


}

