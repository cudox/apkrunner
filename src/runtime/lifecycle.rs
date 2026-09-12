use crate::runtime::state::{RuntimeState, RuntimeEvent};

pub struct ActivityInstance {
    pub name: String,
}

impl ActivityInstance {

    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    pub fn on_create(&self, state: &mut RuntimeState) {
        state.emit(RuntimeEvent::Lifecycle("onCreate".to_string()));
    }

    pub fn on_start(&self, state: &mut RuntimeState) {
        state.emit(RuntimeEvent::Lifecycle("onStart".to_string()));
    }

    pub fn on_resume(&self, state: &mut RuntimeState) {
        state.emit(RuntimeEvent::Lifecycle("onResume".to_string()));
    }
}