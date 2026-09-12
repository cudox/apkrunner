// vm/executor.rs

use crate::vm::{dex::*, frame::Frame};

pub struct Executor;

impl Executor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&mut self, method: &Method) {
        let mut frame = Frame::new(10);

        for ins in &method.code {
            match ins {
                Instruction::LoadConst { dst, value } => {
                    frame.locals[*dst] = *value;
                }

                Instruction::Add { dst, a, b } => {
                    frame.locals[*dst] = frame.locals[*a] + frame.locals[*b];
                }

                Instruction::Sub { dst, a, b } => {
                    frame.locals[*dst] = frame.locals[*a] - frame.locals[*b];
                }

                Instruction::Print { src } => {
                    println!("PRINT -> {}", frame.locals[*src]);
                }
            }
        }
    }
}