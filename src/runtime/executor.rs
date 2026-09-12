use crate::runtime::dex::model::DexFile;
use crate::runtime::vm::VM;

pub struct Executor;

impl Executor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&mut self, dex: DexFile) {
        let mut vm = VM::new();
        vm.run(dex);
    }
}