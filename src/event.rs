#![allow(dead_code)]

#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub(crate) struct EventHeader {
    pub(crate) time: u64,
    pub(crate) core: u16,
}

impl EventHeader {
    pub(crate)fn new() -> Self
    {
        Self {
            time: u64::try_from(Self::get_nano_seconds())
                .expect("Time conversion overflow"),
            core: u16::try_from(nix::sched::sched_getcpu()
                .expect("Could not get cpuID"))
                .expect("cpuid conversion overflow"),
        }
    }

    /// Get nano-seconds since the trace begins for a given timePoint
    fn get_nano_seconds() -> u128 {
        static START_TIMESTAMP: std::sync::OnceLock::<std::time::Instant> = std::sync::OnceLock::<std::time::Instant>::new();
        START_TIMESTAMP.get_or_init(std::time::Instant::now).elapsed().as_nanos()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub(crate) struct EventInfo {
    pub(crate) id: u16,
    pub(crate) value: u32,
}

// Automatic convert tuple to EventInfo
impl std::convert::From<(u16, u32)> for EventInfo {
    fn from(tuple: (u16, u32)) -> Self {
        Self {
            id: tuple.0,
            value: tuple.1,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct EventEntry {
    pub(crate) hdr: EventHeader,
    pub(crate) info: EventInfo,
}

impl EventEntry {
    pub(crate) fn new(id: u16, value: u32) -> Self
    {
        Self {
            hdr: EventHeader::new(),
            info: EventInfo { id, value }
        }
    }
}

// Needed to sort in the heap
impl PartialOrd for EventEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EventEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hdr.time.cmp(&other.hdr.time)
    }
}

impl std::fmt::Display for EventEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "core:{} time:{} id:{} value:{}",
            self.hdr.core, self.hdr.time, self.info.id, self.info.value)
    }
}


#[cfg(test)]
mod profiler {

    use super::*;

    #[test]
    fn eventinfo_construct()
    {
        assert_eq!(EventInfo{id: 2, value: 3}, EventInfo::from((2, 3)));
        assert_eq!(EventInfo{id: 2, value: 3}, (2, 3).into());
    }

    #[test]
    fn eventinfo_order()
    {
        let event_entry1 = EventEntry::new(3, 4);
        let event_entry2 = EventEntry::new(1, 2);

        assert!(event_entry1 < event_entry2);

        let event_clone = event_entry1.clone();
        assert_eq!(event_entry1, event_clone);
    }
}
