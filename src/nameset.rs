#![allow(dead_code)]

use std::io::Write;
use std::str::FromStr;
use std::sync::atomic;
use std::sync::{Arc, RwLock};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

/// A struct representing information about a name, including its file
/// path and line number.
/// This structure represents the entries for events and values
/// individually and is the needed information to generate the pcf
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NameInfo {
    pub(crate) name: String,
    path: std::path::PathBuf,
    line: u32
}

impl NameInfo {
    fn new(name: &String, path: Option<&str>, line: Option<u32>) -> Self
    {
        Self {
            name: name.clone(),
            path: std::path::PathBuf::from_str(path.unwrap_or_default()).expect("Error converting path"),
            line: line.unwrap_or_default()
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
    fn new(name: &String, path: Option<&str>, line: Option<u32>) -> Self
    {
        Self {
            info: NameInfo::new(name, path, line),
            names_values_map: BTreeMap::new()
        }
    }
}

pub(crate) struct NameSet {
    counter: atomic::AtomicU16,
    names_event_map: Arc<RwLock<BTreeMap<u16, NameEntry>>>,
}

impl NameSet {

    const  MAX_USER_EVENT: u16 = u16::MAX / 2;
    const  MAX_EVENT: u16 = u16::MAX;

    pub fn new() -> Self
    {
        Self {
            counter: atomic::AtomicU16::new(Self::MAX_USER_EVENT),
            names_event_map:  Arc::new(RwLock::new(BTreeMap::new()))
        }
    }

    /// Register a new event with event_name and event_id
    /// When event_id is not specified the function generated a new event_it
    /// The generated id is in the internal range (above the user events range)
    pub fn register_event_name(
        &mut self,
        event_name: &str,
        file_name: Option<&str>,
        line: Option<u32>,
        event_id: Option<u16>
    ) -> u16 {
        let real_name: String =
            if event_name.is_empty() {
                format!("{}:{}",file_name.unwrap_or_default(), line.unwrap_or_default())
            } else {
                event_name.to_string()
            };

        let value = NameEntry::new(&real_name, file_name, line);

        // Is the provided id is zero we use the internal events counter.
        let mut event_ref: u16 =
            match event_id {
                Some(evt) => {
                    assert!(evt < Self::MAX_USER_EVENT,
                        "Event value must be < {}", Self::MAX_USER_EVENT);
                    evt
                },
                None => {
                    let last = self.counter.fetch_add(1, atomic::Ordering::Relaxed);
                    assert!(last < Self::MAX_EVENT,
                        "Internal counter event value reached the limit");
                    last + 1
                }
            };

        let mut maplock = self.names_event_map.write().expect("Failed to get name_set lock");

        // Is the event value is already occupied we silently search
        // for the next closest hole and use it. We use the initial
        // value only as a hint.
        match maplock.entry(event_ref) {
            Entry::Vacant(entry) => {entry.insert(value);},
            Entry::Occupied(_) => { // Find the first key available next to event_ref
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
        self.register_event_name(event_name, Some("profiler"), None, None)
    }

    pub fn register_event_value_name(
        &mut self,
        value_name: &str,
        file_name: Option<&str>,
        line: Option<u32>,
        event: u16,
        value: Option<u32>
    ) -> u32 {
        let real_name: String = if value_name.is_empty() {
            format!("{}:{}",file_name.unwrap_or_default(), line.unwrap_or_default())
        } else {
            value_name.to_string()
        };

        let mut maplock = self.names_event_map.write().expect("Failed to get name_set lock");

        match maplock.entry(event) {
            Entry::Vacant(_) => {
                panic!("Cannot register event value: '{}' with id: {} the event ID does not exist.", real_name, event);
            },

            Entry::Occupied(mut entry) => {

                let val = {
                    if let Some(value) = value {
                        value // If specified
                    } else if let Some((&k, _)) = entry.get_mut().names_values_map.iter().next_back() {
                        k + 1 // else get the value after current max
                    } else {
                        1     // else the entry has not values yet and map is empty
                    }
                };


                match entry.get_mut().names_values_map.entry(val) {
                    Entry::Vacant(sub_entry) => {
                        sub_entry.insert(NameInfo::new(&real_name, file_name, line));
                        val
                    },
                    Entry::Occupied(sub_entry) => {
                        panic!("Cannot cannot register event value: '{}' with id {}:{} it is already taken by {}",
                            real_name, event, val, sub_entry.key());
                    },
                }
            },
        }
    }

    pub fn get_event_value_info(
        &self,
        event: u16,
        value: Option<u32>
    ) -> Option<NameInfo> {

        match self
            .names_event_map
            .read()
            .expect("Failed to get name_set read lock")
            .get(&event) {
                None => None,
                Some(entry) => {
                    if let Some(val) = value {
                        entry.names_values_map.get(&val).cloned()
                    } else {
                        Some(entry.info.clone())
                    }
                }
            }
    }

    pub fn create_pcf(&self, trace_dir: &std::path::Path) -> std::io::Result<()>
    {
        let file = std::fs::File::create(trace_dir.join("Trace.pcf"))?;
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
        let mut val = name_set.register_event_name("Event1", Some("File1"), None, Some(1));
        assert_eq!(val, 1);

        val = name_set.register_event_name("Event2", Some("File2"), None, Some(2));
        assert_eq!(val, 2);

        val = name_set.register_event_name("Event3", Some("File3"), Some(0), Some(3));
        assert_eq!(val, 3);

        // Insert with some offset
        val = name_set.register_event_name("Event8.1", Some("File3"), Some(0), Some(8));
        assert_eq!(val, 8);

        // Test the searcher
        val = name_set.register_event_name("Event8.2", Some("File3"), Some(0), Some(8));
        assert_eq!(val, 9);

        // Test the searcher again
        val = name_set.register_event_name("Event8.3", Some("File3"), Some(0), Some(8));
        assert_eq!(val, 10);

        // Test the next in an interleaved hole
        val = name_set.register_event_name("Event1.1", Some("File1"), None, Some(1));
        assert_eq!(val, 4);

        // Test the next in an interleaved hole again 
        val = name_set.register_event_name("Event1.1", Some("File1"), Some(0), Some(2));
        assert_eq!(val, 5);

        assert!(name_set.get_event_value_info(1, None).is_some_and(|info| info.name == "Event1"));
        assert!(name_set.get_event_value_info(2, None).is_some_and(|info| info.name == "Event2"));
        assert!(name_set.get_event_value_info(3, None).is_some_and(|info| info.name == "Event3"));

        assert!(name_set.get_event_value_info(8, None).is_some_and(|info| info.name == "Event8.1"));
        assert!(name_set.get_event_value_info(9, None).is_some_and(|info| info.name == "Event8.2"));
        assert!(name_set.get_event_value_info(10, None).is_some_and(|info| info.name == "Event8.3"));

        assert!(name_set.get_event_value_info(4, None).is_some_and(|info| info.name == "Event1.1"));
        assert!(name_set.get_event_value_info(5, None).is_some_and(|info| info.name == "Event1.1"));

    }

    #[test]
    fn register_event_value_name()
    {
        let mut name_set = NameSet::new();

        // Insert contiguous
        assert_eq!(name_set.register_event_name("Event1", Some("File1"), Some(0), Some(1)), 1);

        let mut val = name_set.register_event_value_name("Value1", Some("File1"), Some(0), 1, Some(1));
        assert_eq!(val, 1);

        val = name_set.register_event_value_name("Value2", Some("File1"), None, 1, Some(2));
        assert_eq!(val, 2);

        val = name_set.register_event_value_name("Value3", Some("File1"), None, 1, Some(3));
        assert_eq!(val, 3);

        assert!(name_set.get_event_value_info(1, Some(1)).is_some_and(|info| info.name == "Value1"));
        assert!(name_set.get_event_value_info(1, Some(2)).is_some_and(|info| info.name == "Value2"));
        assert!(name_set.get_event_value_info(1, Some(3)).is_some_and(|info| info.name == "Value3"));
    }


    #[test]
    fn create_pcf()
    {
        let mut name_set = NameSet::new();

        // Insert contiguous
        assert_eq!(name_set.register_event_name("Event1", Some("File1"), None, Some(1)), 1);
        assert_eq!(name_set.register_event_name("Event2", Some("File2"), None, Some(2)), 2);
        assert_eq!(name_set.register_event_name("Event3", Some("File3"), None, Some(3)), 3);
        assert_eq!(name_set.register_event_name("Event8.1", Some("File3"), Some(0), Some(8)), 8);
        assert_eq!(name_set.register_event_name("Event8.2", Some("File3"), Some(0), Some(8)), 9);
        assert_eq!(name_set.register_event_name("Event8.3", Some("File3"), Some(0), Some(8)), 10);
        assert_eq!(name_set.register_event_name("Event1.1", Some("File1"), Some(0), Some(1)), 4);

        assert_eq!(name_set.register_event_value_name("Value1", Some("File1"), None, 1, Some(1)), 1);
        assert_eq!(name_set.register_event_value_name("Value2", Some("File1"), None, 1, Some(2)), 2);
        assert_eq!(name_set.register_event_value_name("Value3", Some("File1"), None, 1, Some(3)), 3);

        name_set.create_pcf(std::path::Path::new("/tmp")).unwrap();
    }

}
