#![allow(dead_code)]

pub struct Guard {
    /// Event id for this guard. remembered to emit on the destructor
    id: u16,
}

impl Guard {
    pub fn new(id: u16, value: u32) -> Self
    {
        crate::ThreadInfo::emplace_event(id, value);
        Self {id}
    }

    pub fn update(&self, value: u32)
    {
        crate::ThreadInfo::emplace_event(self.id, value);
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        crate::ThreadInfo::emplace_event(self.id, 0);
    }
}



