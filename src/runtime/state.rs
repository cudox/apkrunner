use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum RuntimeEvent {
    Log(String),
    Lifecycle(String),
    StateChange(String, String),
}

#[derive(Debug)]
pub struct RuntimeState {
    pub objects: HashMap<String, String>,
}

impl RuntimeState {

    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
        }
    }

    pub fn emit(&mut self, event: RuntimeEvent) {

        match event {

            RuntimeEvent::Log(msg) => {
                println!("[LOG] {}", msg);
            }

            RuntimeEvent::Lifecycle(msg) => {
                println!("[LIFECYCLE] {}", msg);
            }

            RuntimeEvent::StateChange(k, v) => {
                self.objects.insert(k.clone(), v.clone());
                println!("[STATE] {} = {}", k, v);
            }
        }
    }

    pub fn create_object(&mut self, name: &str) {
        self.objects.insert(name.to_string(), "instance".to_string());

        println!("[OBJECT] created {}", name);
    }
}