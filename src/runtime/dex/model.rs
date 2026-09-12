use crate::runtime::instruction::Instruction;

#[derive(Debug, Clone)]
pub struct DexFile {
    pub methods: Vec<Method>,
}

#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub code: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct CodeItem {
    pub registers: u16,
    pub instructions: Vec<Instruction>,
}

impl DexFile {
    pub fn new(methods: Vec<Method>) -> Self {
        Self { methods }
    }
}