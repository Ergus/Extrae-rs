#![allow(dead_code)]

use perf_event;
use perf_event::events::{Hardware, Software};

use std::collections::{hash_map::Entry, HashMap};

enum SomeEvent {
    Hardware(perf_event::events::Hardware),
    Software(perf_event::events::Software),
    None
}

impl SomeEvent {

    fn build_counter(group: &mut perf_event::Group, event_name: &str) ->std::io::Result<perf_event::Counter>
    {
        match Self::event_from_str(event_name) {
            SomeEvent::Hardware(hw) => group.add(&perf_event::Builder::new(hw)),
            SomeEvent::Software(sw) => group.add(&perf_event::Builder::new(sw)),
            SomeEvent::None => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid event name {}", event_name),
            )),
        }
    }

    fn event_from_str(event_name: &str) -> SomeEvent
    {
        match event_name {
            "cycles" => SomeEvent::Hardware(Hardware::CPU_CYCLES),
            "instructions" => SomeEvent::Hardware(Hardware::INSTRUCTIONS),
            "cache-references" => SomeEvent::Hardware(Hardware::CACHE_REFERENCES),
            "cache-misses" => SomeEvent::Hardware(Hardware::CACHE_MISSES),
            "page-faults" => SomeEvent::Software(Software::PAGE_FAULTS),
            "context-switches" => SomeEvent::Software(Software::CONTEXT_SWITCHES),
            "cpu-migrations" => SomeEvent::Software(Software::CPU_MIGRATIONS),
            _ => SomeEvent::None, // Unknown event
        }
    }

}

pub(crate) struct PerfManager {
    pub(crate) group: perf_event::Group,
    perf_to_extrae_map: HashMap<u64, u16>,
    perf_counters: Vec<perf_event::Counter>,
}

impl PerfManager {
    pub(crate) fn new(event_names: &[&str]) -> Option<Self>
    {
        if event_names.is_empty() {
            return None;
        }

        let mut group = perf_event::Group::new().expect("Cannot build event group");

        let mut perf_to_extrae_map = HashMap::<u64, u16>::new();

        let mut perf_counters: Vec<perf_event::Counter> = Vec::<perf_event::Counter>::new();

        for event_name in event_names {

            let perf_counter = SomeEvent::build_counter(&mut group, event_name);

            match perf_counter {
                Ok(perf_counter) => {

                    let extrae_id = crate::GlobalInfo::register_event_name(
                        event_name,
                        None,
                        None,
                        None
                    );

                    match perf_to_extrae_map.entry(perf_counter.id()) {
                        Entry::Vacant(entry) => entry.insert(extrae_id),
                        Entry::Occupied(_) => panic!("Cannot insert event because it's id already exist"),
                    };


                    perf_counters.push(perf_counter);
                },
                Err(error) => eprintln!("{}", error)
            }
        }

        group.reset().expect("Failed to initialize counters");
        group.enable().expect("Error enabling counters.");

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
