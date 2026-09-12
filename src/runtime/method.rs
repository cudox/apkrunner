use crate::runtime::instruction::Instruction;

#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub code: Vec<Instruction>,
}

impl Method {
    pub fn new(name: &str, code: Vec<Instruction>) -> Self {
        Self {
            name: name.to_string(),
            code,
        }
    }
}