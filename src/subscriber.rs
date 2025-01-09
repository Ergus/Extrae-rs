#![allow(dead_code)]

use tracing::{span, Event, Metadata, Subscriber};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// This is a helper container protected with an rwlock.
/// The main goal for this class is that the new entries requires to
/// take the write lock. But the most frequent accesses are read
/// operations that require only the read lock.
/// The default operation for this is usually get().or_insert(). which
/// enforces to take the write lock.
/// This class only takes the write lock when a new entry will be inserted.
/// But the check is only with the read lock, which significantly
/// reduces overhead.
#[derive(Default)]
struct SubscriberContainer<K, V, S =  std::hash::RandomState> {
    rwmap: Arc<RwLock<HashMap<K, V, S>>>, // Store span IDs and names
}

impl<K: Eq + std::hash::Hash + Clone,
    V: Clone> SubscriberContainer<K, V,  std::hash::RandomState> {
    pub fn new() -> Self
    {
        Self {
            rwmap: Arc::new(RwLock::new(HashMap::new()))
        }
    }

    pub fn get_or_insert_with<F: FnOnce() -> V>(
        &self,
        key: &K,
        default: F
    ) -> V {

        {
            let read_lock = self.rwmap.read().expect("Couldn't get read subscriber");
            if let Some(existing_value) = read_lock.get(&key) {
                return existing_value.clone();
            }
        }

        // Take a write lock if the key does not exist
        let mut write_guard = self.rwmap.write().expect("Couldn't get write subscriber");
        write_guard.entry(key.clone()).or_insert_with(default).clone()
    }
}

/// Implement a tracing subscriber to emit extrae events.
/// The subscriber is useful because the tokio crate is already integrated
/// with the tracing crate and emit events when a task starts and end,
/// but this also works with the tracing defined macros.
pub struct ExtraeSubscriber {
    tokio_event_id: u16,
    spans: SubscriberContainer<String, u16>,
    events: SubscriberContainer<String, u32>,
}

impl ExtraeSubscriber {
    pub fn new() -> Self {
        let tokio_event_id = crate::GlobalInfo::register_event_name("tokio_event", None, None, None);
        crate::ThreadInfo::with(|_| {});
        Self {
            tokio_event_id,
            spans: SubscriberContainer::default(),
            events: SubscriberContainer::default(),
        }
    }
}

impl Subscriber for ExtraeSubscriber {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    /// Get the span id from
    fn new_span(&self, attrs: &span::Attributes<'_>) -> span::Id {

        let name = attrs.metadata().name().to_string();

        let id: u16 = self.spans.get_or_insert_with(
            &name,
            || {
                crate::GlobalInfo::register_event_name(
                    &name,
                    attrs.metadata().file(),
                    attrs.metadata().line(),
                    None
                )
            }
        );

        span::Id::from_u64(id.into())
    }

    fn record(&self, _id: &span::Id, _values: &span::Record)
    {
        // let mut visitor = EventVisitor::default();
        // values.record(&mut visitor);

        // let event: u16 = id.into_u64() as u16;

        // let value: u32 = visitor.value.expect("Record requires a value in the record! call");

        // match crate::GlobalInfo::get_event_value_info(event, Some(value)) {
        //     Some(info) => {
        //         assert_eq!(info.name, visitor.message.unwrap_or("".to_string()));
        //     },
        //     None => {
        //         crate::GlobalInfo::register_event_value_name(
        //             visitor.message.unwrap_or("".to_string()).as_str(),
        //             None,
        //             None,
        //             event,
        //             Some(value)
        //         );
        //     }
        // };

        // crate::ThreadInfo::emplace_event(event, value);
    }

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {
        // Handle parent/child relationships
    }

    /// This is emitted with the info! macro
    /// Every event receives a value id and is emitted with the
    /// tokio_event_id.
    /// The event value can be specified with the value keyword-key:
    /// info!(value = 5, "My event message")
    fn event(&self, event: &Event<'_>) {

        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let evt_name = visitor
            .message
            .unwrap_or_else(|| event.metadata().name().to_string());

        // Get a value or generate a new one
        let value = self.events.get_or_insert_with(
            &event.metadata().name().to_string(),
            || {
                crate::GlobalInfo::register_event_value_name(
                    evt_name.as_str(),
                    event.metadata().file(),
                    event.metadata().line(),
                    self.tokio_event_id,
                    visitor.value // When the value is None, the function generated a new value
                )
            }
        );

        crate::ThreadInfo::emplace_event(self.tokio_event_id, value);

        // This is TODO work. at the moment non-critical.
        //println!("Event recorded: {:?}", event);

    }

    fn enter(&self, id: &span::Id) {
        crate::ThreadInfo::emplace_event(id.into_u64() as u16, 1);
    }

    fn exit(&self, id: &span::Id) {
        crate::ThreadInfo::emplace_event(id.into_u64() as u16, 0);
    }
}

#[derive(Default)]
struct EventVisitor {
    message: Option<String>,
    value: Option<u32>
}

impl tracing::field::Visit for EventVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug)
    {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value));
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64)
    {
        match field.name() {
            "value" => self.value = Some(value as u32),
            _ => {}
        }
    }
}
