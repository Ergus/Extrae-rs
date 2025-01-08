#![allow(dead_code)]

use tracing::{span, Event, Metadata, Subscriber};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};


pub struct ExtraeSubscriber {
    subscriber_id: u16,
    spans: Arc<RwLock<HashMap<String, u16>>>, // Store span IDs and names
}

impl ExtraeSubscriber {
    pub fn new() -> Self {
        let subscriber_id = crate::GlobalInfo::register_event_name("subscriber_created", "", 0, 0);
        crate::ThreadInfo::emplace_event(subscriber_id, 1);
        Self {
            subscriber_id,
            spans: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Drop for ExtraeSubscriber {
    fn drop(&mut self) {
        crate::ThreadInfo::emplace_event(self.subscriber_id, 0);
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

        let read_lock = self.spans.read().expect("Couldn't get read subscriber");

        let id: u16 = match read_lock.get(name) {
                Some(id) => *id,
                None => {
                    drop(read_lock);

                    let id = crate::GlobalInfo::register_event_name(name, file, line, 0);
                    self.spans.write().expect("Couldn't get write subscriber").insert(name.to_string(), id);
                    id
                },
            };

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
        crate::ThreadInfo::emplace_event(id.into_u64() as u16, 0);
    }
}


#[cfg(test)]
mod tests {
}
