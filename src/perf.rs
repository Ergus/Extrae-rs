#![allow(dead_code)]

use perf_event;
use perf_event::events::{Hardware, Software};

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

struct EventInfo {
    event: perf_event::Counter,
    extrae_id: u16,
}

pub(crate) struct PerfManager {
    pub(crate) group: perf_event::Group,
    events_info: Vec<EventInfo>,
}

impl PerfManager {
    pub(crate) fn new(input_info: &Vec<(String, u16)>) -> Option<Self>
    {
        if input_info.is_empty() {
            return None;
        }

        let mut group = perf_event::Group::new().expect("Cannot build event group");

        let mut events_info: Vec<EventInfo> = Vec::<EventInfo>::with_capacity(input_info.len());

        for (event_name, extrae_id) in input_info {

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
                    events_info.push(
                        EventInfo{
                            event: perf_counter,
                            extrae_id: *extrae_id
                        });
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
        assert_eq!(events_info.len(), events_info.len());

        Some(Self{group, events_info})
    }

    /// Add a software or hardware event.
    pub(crate) fn get_counters(&mut self) -> Vec<(u16, u32)>
    {
        // Read the counter values
        let entries = self.group.read().expect("Failed reading counters.");
        assert_ne!(entries.len(), 0);

        entries.iter()
            .zip(&self.events_info)
            .filter_map(|(entry, event_info)| {
                assert_eq!(entry.id(), event_info.event.id());

                match entry.value().try_into() {
                    Ok(value) if value == 0 => None,
                    Ok(value) => Some((event_info.extrae_id, value)),
                    Err(e) => panic!("Overflow in event to value conversion: {:?}", e),
                }
            }).collect()
    }

}
