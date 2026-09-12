use std::collections::HashMap;

#[derive(Debug)]
pub struct ClassRegistry {

    pub classes: HashMap<String, bool>,
}

impl ClassRegistry {

    pub fn new() -> Self {

        Self {
            classes: HashMap::new(),
        }
    }

    pub fn register(&mut self, class: &str) {

        println!("Registering class: {}", class);

        self.classes.insert(class.to_string(), true);
    }

    pub fn exists(&self, class: &str) -> bool {

        self.classes.get(class).copied().unwrap_or(false)
    }
}