#![allow(dead_code)]

use std::io::Write;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

/// A struct representing information about a name, including its file
/// path and line number.
#[derive(Debug, Clone, PartialEq)]
struct NameInfo {
    name: String,
    path: std::path::PathBuf,
    line: u32
}

impl NameInfo {
    fn new(name: &str, path: &str, line: u32) -> Self
    {
        Self {
            name: name.to_string(),
            path: std::path::PathBuf::from_str(path).expect("Error converting path"),
            line
        }
    }
}

/// Implementing the Display trait to convert the struct to a string
impl std::fmt::Display for NameInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} ({}:{})", self.name, self.path.to_str().unwrap(), self.line)
    }
}

struct NameEntry {
    info: NameInfo,
    names_values_map: BTreeMap<u32, NameInfo>,
}

impl NameEntry {
    fn new(name: &str, path: &str, line: u32) -> Self
    {
        Self {
            info: NameInfo::new(name, path, line),
            names_values_map: BTreeMap::new()
        }
    }
}

pub struct NameSet {
    counter: u16,
    names_event_map: Arc<RwLock<BTreeMap<u16, NameEntry>>>,
}

impl NameSet {

    const  MAX_USER_EVENT: u16 = u16::MAX / 2;
    const  MAX_EVENT: u16 = u16::MAX;

    pub fn new() -> Self
    {
        Self {
            counter: Self::MAX_USER_EVENT,
            names_event_map:  Arc::new(RwLock::new(BTreeMap::new()))
        }
    }

    pub fn register_event_name(
        &mut self,
        event_name: &str,
        file_name: &str,
        line: u32,
        event: u16
    ) -> u16 {
        let real_name = if event_name.is_empty() {
            format!("{}:{}",file_name, line)
        } else {
            event_name.to_string()
        };

        let value = NameEntry::new(real_name.as_str(), file_name, line);

        let mut event_ref: u16 =
            if event == u16::default() {
                self.counter += 1;
                self.counter
            } else {
                event
            };

        let mut maplock = self.names_event_map.write().expect("Failed to get name_set lock");

        match maplock.entry(event_ref) {
            Entry::Vacant(entry) => {entry.insert(value);},
            Entry::Occupied(_) => { // Find the first key available
                for (&existing_key, _) in maplock.range(event_ref..).by_ref() {
                    if existing_key != event_ref {
                        // Found a hole
                        break;
                    }
                    event_ref += 1;
                }

                maplock.insert(event_ref, value);
            },
        };

        event_ref
    }

    pub fn register_event_name_internal (
        &mut self,
        event_name: &str
    ) -> u16 {
        self.register_event_name(event_name, "profiler", 0, 0)
    }

    pub fn register_value_name(
        &mut self,
        value_name: &str,
        file_name: &str,
        line: u32,
        event: u16,
        value: u32
    ) -> u32 {
        let real_name = if value_name.is_empty() {
            format!("{}:{}",file_name, line)
        } else {
            value_name.to_string()
        };

        let mut maplock = self.names_event_map.write().expect("Failed to get name_set lock");

        match maplock.entry(event) {
            Entry::Vacant(_) => {
                panic!("Cannot register event value: '{}' with id: {} the event ID does not exist.", real_name, value);
            },

            Entry::Occupied(mut entry) => {
                match entry.get_mut().names_values_map.entry(value) {
                    Entry::Vacant(sub_entry) => {
                        sub_entry.insert(NameInfo::new(real_name.as_str(), file_name, line));
                        value
                    },
                    Entry::Occupied(sub_entry) => {
                        panic!("Cannot cannot register event value: '{}' with id {}:{} it is already taken by {}",
                            real_name, event, value, sub_entry.key());
                    },
                }
            },
        }
    }

    pub fn create_pcf(&self, trace_dir: &std::path::Path) -> std::io::Result<()>
    {
        let file = std::fs::File::create(trace_dir.join("Trace.pcf")).unwrap();
        let mut writer = std::io::BufWriter::new(file);

        let mapread = self.names_event_map.read().expect("Failed to get name_set lock");

        for (key, &ref name_entry) in mapread.iter() {
            writeln!(writer, "# {}:{}", name_entry.info.path.to_str().unwrap(), name_entry.info.line)?;
            writeln!(writer, "EVENT_TYPE")?;
            writeln!(writer, "0 {} {}", key, name_entry.info.name)?;

            if !name_entry.names_values_map.is_empty() {
                writeln!(writer, "VALUES")?;

                for (key, &ref value_entry) in name_entry.names_values_map.iter() {
                    writeln!(writer, "{} {}:{}", key, name_entry.info.name, value_entry.name)?;
                }
            }

            writeln!(writer, "")?;
        }

        Ok(())
    }
}


#[cfg(test)]
mod nameset{

    use super::*;

    #[test]
    fn register_event_names()
    {
        let mut name_set = NameSet::new();

        // Insert contiguous
        let mut val = name_set.register_event_name("Event1", "File1", 0, 1);
        assert_eq!(val, 1);

        val = name_set.register_event_name("Event2", "File2", 0, 2);
        assert_eq!(val, 2);

        val = name_set.register_event_name("Event3", "File3", 0, 3);
        assert_eq!(val, 3);

        // Insert with some offset
        val = name_set.register_event_name("Event8.1", "File3", 0, 8);
        assert_eq!(val, 8);

        // Test the searcher
        val = name_set.register_event_name("Event8.2", "File3", 0, 8);
        assert_eq!(val, 9);

        // Test the searcher again
        val = name_set.register_event_name("Event8.3", "File3", 0, 8);
        assert_eq!(val, 10);

        // Test the next in an interleaved hole
        val = name_set.register_event_name("Event1.1", "File1", 0, 1);
        assert_eq!(val, 4);

        // Test the next in an interleaved hole again 
        val = name_set.register_event_name("Event1.1", "File1", 0, 2);
        assert_eq!(val, 5);


    }

    #[test]
    fn register_value_name()
    {
        let mut name_set = NameSet::new();

        // Insert contiguous
        assert_eq!(name_set.register_event_name("Event1", "File1", 0, 1), 1);

        let mut val = name_set.register_value_name("Value1", "File1", 0, 1, 1);
        assert_eq!(val, 1);

        val = name_set.register_value_name("Value2", "File1", 0, 1, 2);
        assert_eq!(val, 2);

        val = name_set.register_value_name("Value3", "File1", 0, 1, 3);
        assert_eq!(val, 3);
    }


    #[test]
    fn create_pcf()
    {
        let mut name_set = NameSet::new();

        // Insert contiguous
        assert_eq!(name_set.register_event_name("Event1", "File1", 0, 1), 1);
        assert_eq!(name_set.register_event_name("Event2", "File2", 0, 2), 2);
        assert_eq!(name_set.register_event_name("Event3", "File3", 0, 3), 3);
        assert_eq!(name_set.register_event_name("Event8.1", "File3", 0, 8), 8);
        assert_eq!(name_set.register_event_name("Event8.2", "File3", 0, 8), 9);
        assert_eq!(name_set.register_event_name("Event8.3", "File3", 0, 8), 10);
        assert_eq!(name_set.register_event_name("Event1.1", "File1", 0, 1), 4);

        assert_eq!(name_set.register_value_name("Value1", "File1", 0, 1, 1), 1);
        assert_eq!(name_set.register_value_name("Value2", "File1", 0, 1, 2), 2);
        assert_eq!(name_set.register_value_name("Value3", "File1", 0, 1, 3), 3);

        name_set.create_pcf(std::path::Path::new("/tmp")).unwrap();
    }

}
