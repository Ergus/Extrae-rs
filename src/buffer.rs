#![allow(dead_code)]

use crate::bufferinfo;

pub struct Buffer {
    name: String,
    path: std::path::PathBuf,
    file: Option<std::fs::File>,
    info: bufferinfo::BufferInfo,
}

impl Buffer {

    pub fn new(
        id: u32,
        tid: &std::thread::ThreadId,
        name: &str,
        path: std::path::PathBuf,
        start_gtime: &std::time::Duration
    ) -> Self {
        Self {
            name: name.to_string(),
            path,
            file: None,
            info: bufferinfo::BufferInfo::new(id, &tid, &start_gtime)
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

    pub fn name(&self) -> &str
    {
        self.name.as_str()
    }

    fn from_path(path: std::path::PathBuf) -> Self
    {
        let mut file = std::fs::File::open(&path).unwrap();
        let name = path.as_os_str().to_str().unwrap().to_string();

        let info = bufferinfo::BufferInfo::from_file(&mut file);

        Self { name, path, file: None, info }
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

    #[test]
    fn buffer_construct_destruct()
    {
        let path = std::path::PathBuf::from_str("/tmp/buffer_construct_destruct").unwrap();

        let mut buff = Buffer::new(
            1,
            &std::thread::current().id(),
            "",
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
            "",
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
            "",
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
                "",
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
            assert_eq!(imported_info.entries[i].info.id, i as u16);
            assert_eq!(imported_info.entries[i].info.value, (i + 1) as u32);
        }

        std::fs::remove_file(path).unwrap();
    }
}
