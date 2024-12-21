#![allow(dead_code)]

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
    line: usize
}

/// Implementing the Display trait to convert the struct to a string
impl std::fmt::Display for NameInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} ({}:{})", self.name, self.path.to_str().unwrap(), self.line)
    }
}

struct NameEntry {
    info: NameInfo,
    names_values_map: BTreeMap<u16, NameInfo>,
}

impl NameEntry {
    fn new(name: &str, path: &str, line: usize) -> Self
    {
        Self {
            info: NameInfo {
                name: name.to_string(),
                path: std::path::PathBuf::from_str(path).expect("Error converting path"),
                line
            },
            names_values_map: BTreeMap::new()
        }
    }
}

struct NameSet {
    counter: u16,
    names_event_map: Arc<RwLock<BTreeMap<u16, NameEntry>>>,
}

impl NameSet {

    const  MAX_USER_EVENT: u16 = u16::MAX / 2;
    const  MAX_EVENT: u16 = u16::MAX;

    fn new() -> Self
    {
        Self {
            counter: Self::MAX_USER_EVENT,
            names_event_map:  Arc::new(RwLock::new(BTreeMap::new()))
        }
    }

    fn register_event_name(&mut self, event_name: &str, file_name: &str, line: usize, event: u16) -> u16
    {
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

        // Test the searcher
        val = name_set.register_event_name("Event8.3", "File3", 0, 8);
        assert_eq!(val, 10);

        // Test the next in an interleaved hole
        val = name_set.register_event_name("Event1.1", "File1", 0, 1);
        assert_eq!(val, 4);


    }


}
