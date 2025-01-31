#![allow(dead_code)]

use perf_event;
use perf_event::events::{Hardware, Software};

use std::collections::{hash_map::Entry, HashMap};

#[derive(Debug, Clone, Copy)]
pub(crate) enum SomeEvent {
    Hardware(perf_event::events::Hardware),
    Software(perf_event::events::Software),
    None
}

impl SomeEvent {

    pub(crate) const EVENTS_LIST: [(&str, SomeEvent); 15] = [
        ("cycles", SomeEvent::Hardware(Hardware::CPU_CYCLES)),
        ("instructions", SomeEvent::Hardware(Hardware::INSTRUCTIONS)),
        ("cache-references", SomeEvent::Hardware(Hardware::CACHE_REFERENCES)),
        ("cache-misses", SomeEvent::Hardware(Hardware::CACHE_MISSES)),
        ("branch-instructions", SomeEvent::Hardware(Hardware::BRANCH_INSTRUCTIONS)),
        ("branch-misses", SomeEvent::Hardware(Hardware::BRANCH_MISSES)),
        ("bus-cycles", SomeEvent::Hardware(Hardware::BUS_CYCLES)),
        ("stalled-cycles-frontend", SomeEvent::Hardware(Hardware::STALLED_CYCLES_FRONTEND)),
        ("stalled-cycles-backend", SomeEvent::Hardware(Hardware::STALLED_CYCLES_BACKEND)),
        ("ref-cpu-cycles", SomeEvent::Hardware(Hardware::REF_CPU_CYCLES)),

        ("page-faults", SomeEvent::Software(Software::PAGE_FAULTS)),
        ("context-switches", SomeEvent::Software(Software::CONTEXT_SWITCHES)),
        ("cpu-migrations", SomeEvent::Software(Software::CPU_MIGRATIONS)),
        ("page-faults-min", SomeEvent::Software(Software::PAGE_FAULTS_MIN)),
        ("page-faults-maj", SomeEvent::Software(Software::PAGE_FAULTS_MAJ)),
    ];

    pub(crate) fn event_from_str(event_name: &str) -> SomeEvent
    {
        Self::EVENTS_LIST
            .iter()
            .find(|x| x.0 == event_name)
            .map(|x| x.1)
            .unwrap_or(SomeEvent::None)
    }

}

pub(crate) struct PerfManager {
    pub(crate) group: perf_event::Group,
    perf_to_extrae_map: HashMap<u64, u16>,
    perf_counters: Vec<perf_event::Counter>,
}

impl PerfManager {
    pub(crate) fn new(events_info: &Vec<(String, u16)>) -> Option<Self>
    {
        if events_info.is_empty() {
            return None;
        }

        let mut group = perf_event::Group::new().expect("Cannot build event group");

        let mut perf_to_extrae_map = HashMap::<u64, u16>::new();

        let mut perf_counters: Vec<perf_event::Counter> = Vec::<perf_event::Counter>::new();

        for (event_name, extrae_id) in events_info {

            let perf_counter = {

                match SomeEvent::event_from_str(event_name.as_str()) {
                    SomeEvent::Hardware(hw) => group.add(&perf_event::Builder::new(hw)),
                    SomeEvent::Software(sw) => group.add(&perf_event::Builder::new(sw)),
                    SomeEvent::None => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("Invalid event name {}", event_name),
                    )),
                }
            };

            match perf_counter {
                Ok(perf_counter) => {

                    match perf_to_extrae_map.entry(perf_counter.id()) {
                        Entry::Vacant(entry) => entry.insert(*extrae_id),
                        Entry::Occupied(_) => panic!("Cannot insert event because it's id already exist"),
                    };


                    perf_counters.push(perf_counter);
                },
                Err(error) => eprintln!("{}", error)
            }
        }

        assert_eq!(group.read().unwrap().len(), events_info.len());

        group.reset().expect("Failed to initialize counters");
        group.enable().expect("Error enabling counters.");

        // Check that all the counters were enabled
        assert_eq!(group.read().unwrap().len(), events_info.len());

        // Check translation info.
        assert_eq!(perf_counters.len(), events_info.len());
        assert_eq!(perf_to_extrae_map.len(), events_info.len());

        Some(Self{ group,  perf_to_extrae_map, perf_counters})
    }

    /// Add a software or hardware event.
    pub(crate) fn get_counters(&mut self) -> Vec<(u16, u32)>
    {
        // Read the counter values
        let entries = self.group.read().expect("Failed reading counters.");
        assert_ne!(entries.len(), 0);

        let mut res = Vec::<(u16, u32)>::new();

        for entry in entries.iter() {
            let perf_id = entry.id();
            let extrae_id = self.perf_to_extrae_map
                .get(&perf_id)
                .expect("Internal profiler error");

            let value = entry.value();
            res.push((extrae_id.clone(), value.try_into().expect("Overflow in event to value conversion")));
        }
        res
    }

}
