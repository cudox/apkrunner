use crate::runtime::instruction::Instruction;

pub fn decode(insns: &[u16]) -> Vec<Instruction> {
    let mut out = Vec::new();
    let mut i = 0;

    while i < insns.len() {
        let unit = insns[i];
        let opcode = (unit & 0xff) as u8;

        match opcode {

            0x00 => {
                out.push(Instruction::Nop);
                i += 1;
            }

            0x12 => {
                let dst = ((unit >> 8) & 0x0f) as u8;
                let value = ((unit >> 12) as i8) as i32;
                out.push(Instruction::Const { dst, value });
                i += 1;
            }

            0x01 => {
                let dst = ((unit >> 8) & 0x0f) as u8;
                let src = ((unit >> 12) & 0x0f) as u8;
                out.push(Instruction::Move { dst, src });
                i += 1;
            }

            0x0f => {
                let r = ((unit >> 8) & 0x0f) as u8;
                out.push(Instruction::Return { value: Some(r) });
                i += 1;
            }

            0x0e => {
                out.push(Instruction::ReturnVoid);
                i += 1;
            }

            _ => {
                out.push(Instruction::Unknown(unit));
                i += 1;
            }
        }
    }

    out
}