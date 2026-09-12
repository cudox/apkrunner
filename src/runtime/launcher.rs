use crate::runtime::state::{RuntimeState, RuntimeEvent};

pub fn launch_activity(state: &mut RuntimeState, activity: &str) {

    state.create_object(activity);

    state.emit(RuntimeEvent::Lifecycle(
        format!("Launching {}", activity)
    ));
}