use crate::runtime::dex::model::DexFile;
use crate::runtime::instruction::Instruction;

pub struct VM {
    pub regs: Vec<u8>,
    pc: usize,
}

impl VM {
    pub fn new() -> Self {
        Self {
            regs: vec![0; 256],
            pc: 0,
        }
    }

    pub fn run(&mut self, dex: DexFile) {
        let mut pc: usize = 0;
        let instructions: Vec<Instruction> = dex.methods
            .iter()
            .flat_map(|m| m.code.iter().cloned())
            .collect();

        while pc < instructions.len() {
            let ins = instructions[pc].clone();
            let jumped = self.execute(ins, pc);
            if jumped {
                pc = self.pc;
                continue;
            }
            pc += 1;
        }
    }

    fn execute(&mut self, ins: Instruction, pc: usize) -> bool {
        match ins {
            Instruction::Nop => {}

            Instruction::Const { dst, value } => {
                self.regs[dst as usize] = value as u8;
            }

            Instruction::Move { dst, src } => {
                self.regs[dst as usize] = self.regs[src as usize];
            }

            Instruction::Add { dst, a, b } => {
                self.regs[dst as usize] =
                    self.regs[a as usize] + self.regs[b as usize];
            }

            Instruction::Sub { dst, a, b } => {
                self.regs[dst as usize] =
                    self.regs[a as usize].saturating_sub(self.regs[b as usize]);
            }

            Instruction::IfEq { a, b, target } => {
                if self.regs[a as usize] == self.regs[b as usize] {
                    self.pc = target as usize;
                    return true;
                }
            }

            Instruction::Print { src } => {
                println!("{}", self.regs[src as usize]);
            }

            Instruction::Return { .. } => {
                return true;
            }

            Instruction::ReturnVoid => {
                return true;
            }

            Instruction::Unknown(op) => {
                println!("unknown opcode {}", op);
            }
        }
        false
    }
}