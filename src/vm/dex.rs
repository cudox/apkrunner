// vm/dex.rs

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
pub enum Instruction {
    LoadConst { dst: usize, value: i32 },
    Add { dst: usize, a: usize, b: usize },
    Sub { dst: usize, a: usize, b: usize },
    Print { src: usize },
}