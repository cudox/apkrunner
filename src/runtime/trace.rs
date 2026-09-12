use crate::runtime::state::{RuntimeState, RuntimeEvent};

pub struct TraceEngine;

impl TraceEngine {

    pub fn boot(state: &mut RuntimeState) {

        state.emit(RuntimeEvent::Log(
            "BOOT: Runtime started".to_string(),
        ));
    }

    pub fn load_dex(state: &mut RuntimeState, dex: &str) {

        state.emit(RuntimeEvent::Log(
            format!("DEX: loading {}", dex),
        ));
    }

    pub fn execute_activity(state: &mut RuntimeState, class: &str) {

        state.emit(RuntimeEvent::Log(
            format!("CLASS: {}", class),
        ));

        state.emit(RuntimeEvent::Log(
            "EXEC: onCreate()".to_string(),
        ));

        state.emit(RuntimeEvent::StateChange(
            "activity_started".to_string(),
            "true".to_string(),
        ));

        state.emit(RuntimeEvent::Log(
            "EXEC: onStart()".to_string(),
        ));

        state.emit(RuntimeEvent::Log(
            "EXEC: onResume()".to_string(),
        ));
    }

    pub fn finish(state: &mut RuntimeState) {

        state.emit(RuntimeEvent::Log(
            "END: execution finished".to_string(),
        ));
    }
}