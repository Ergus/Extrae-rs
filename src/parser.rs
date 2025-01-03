#![allow(dead_code)]

use std::io::{Read, Write};
use std::fs::File;
use std::iter::Iterator;

use chrono::TimeZone;

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

pub(crate) struct Merger
{
    dir_path: std::path::PathBuf,
    file_paths: Vec<std::path::PathBuf>,
    events: Vec<ExtendedEvent>,
    threads: std::collections::BTreeSet<u32>,
    cores: std::collections::BTreeSet<u16>,
    start_global_time: u64
}

impl Merger {

    pub(crate) fn new(dir: &std::path::Path) -> Self
    {
        let file_paths = Merger::get_files_with_extension(dir, "bin");
        let (events, threads, cores, start_global_time)
            = Merger::merge_files(&file_paths);

        Self {
            dir_path: std::path::PathBuf::from(dir),
            file_paths,
            events, threads, cores, start_global_time
        }
    }


    /// Get a vector of paths for all the files with a given extension
    /// inside the given path.
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

    // This creates a Paraver trace from the merged information.
    pub(crate) fn create_prv(&self, trace_dir: &std::path::Path) -> std::io::Result<()>
    {
        assert!(!self.events.is_empty(), "The events list is empty");

        let file = std::fs::File::create(trace_dir.join("Trace.prv"))?;
        let mut writer = std::io::BufWriter::new(file);

        // Convert u64 timestamp to DateTime<Utc>
        let datetime: chrono::DateTime<chrono::Local>
            = chrono::Local.timestamp_opt(self.start_global_time as i64, 0).unwrap();

        // Print the header
        writeln!(
            writer,
            "#Paraver ({}):{}_ns:1({}):1:1({}:1)",
            datetime.format("%d/%m/%Y at %H:%M"),
            self.events.last().unwrap().time - self.events.first().unwrap().time,
            self.cores.iter().max().unwrap(),
            self.threads.len()
        )?;

        println!("Cores: {:?}", self.cores);
        println!("Threads: {:?}", self.threads);
        println!("Total Events: {}", self.events.len());

        for event in self.events.iter() {
            writeln!(writer, "{}", event)?;
        }

        Ok(())
    }


    /// This function merges multiple trace files into a single
    /// sequential buffer.  The entries in the output are sorted by
    /// the entry timestamp and consecutive events with same timestamp
    /// from the same thread are grouped.
    ///
    /// The parsing process uses a new method totally different from
    /// the ones in ExtraeWin (loading the files in memory and merging
    /// by pairs)
    ///
    /// This function instead opens all the trace files
    /// simultaneously.  The first entries in every thread are saved
    /// in a reversed priority queue (BinaryHeap) with 1
    /// entry/thread.
    ///
    /// The principal loop requests the next entry from the BinaryHeap
    /// (the one with the lower timestamp) and restores it with the
    /// next one from the same trace file.
    ///
    /// The TraceIterator class use std::io::BufReader to reduce
    /// system call and improve read speed.
    fn merge_files(
        file_paths: &Vec<std::path::PathBuf>
    ) -> (Vec<ExtendedEvent>,
          std::collections::BTreeSet<u32>,
          std::collections::BTreeSet<u16>,
          u64
    ) {
        let mut heap = std::collections::BinaryHeap::new();

        let mut trace_iters: Vec<_>
            = file_paths
                .iter()
                .map(|path| TraceIterator::open(path.as_path()))
                .collect();

        let total_events: u32 = trace_iters.iter().map(|item| item.header.total_flushed).sum();

        let all_equal = trace_iters.windows(2).all(|pair| pair[0].header.start_gtime == pair[1].header.start_gtime);
        assert!(all_equal, "Some global time differs in trace headers");

        let start_time: u64 = trace_iters[0].header.start_gtime;

        let mut events = Vec::<ExtendedEvent>::with_capacity(total_events as usize);
        let mut cores = std::collections::BTreeSet::<u16>::new();
        let threads: std::collections::BTreeSet::<u32>
            = trace_iters.iter().map(|item| item.header.id).collect();

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
            cores.insert(ext_entry.core);
            events.push(ext_entry);
        }

        assert_eq!(total_events, counter);

        (events, threads, cores, start_time)
    }
}

