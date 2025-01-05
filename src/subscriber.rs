#![allow(dead_code)]

use tracing::{span, Event, Metadata, Subscriber};

pub struct ExtraeSubscriber {
}

impl ExtraeSubscriber {
    fn new() -> Self {
        let _ = crate::GlobalInfo::as_ref();
        Self {}
    }
}

impl Subscriber for ExtraeSubscriber {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        // Decide whether this metadata should be recorded
        //metadata.level() <= &tracing::Level::INFO
        true
    }

    fn new_span(&self, attrs: &span::Attributes<'_>) -> span::Id {

        let name = attrs.metadata().name();
        let file = attrs.metadata().file().or(Some("Unknown")).unwrap();
        let line = attrs.metadata().line().or(Some(0)).unwrap();

        let id = crate::GlobalInfo::register_event_name(name, file, line, 0);

        println!("New span created: {} with ID {:?}", name, id);
        span::Id::from_u64(id.into())

    }

    fn record(&self, _: &span::Id, _: &span::Record<'_>) {
        // Handle span updates
    }

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {
        // Handle parent/child relationships
    }

    fn event(&self, event: &Event<'_>) {
        // Custom handling of events
        println!("Event recorded: {:?}", event);

        // Example: Increment a counter on each event
        // let mut state = self.state.lock().unwrap();
        // *state += 1;
    }

    fn enter(&self, id: &span::Id) {
        crate::ThreadInfo::emplace_event(id.into_u64() as u16, 1);
    }

    fn exit(&self, id: &span::Id) {
        crate::ThreadInfo::emplace_event(id.into_u64() as u16, 1);
    }
}
