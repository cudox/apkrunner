#[derive(Debug, Clone)]
pub enum Instruction {
    Nop,

    Const {
        dst: u8,
        value: i32,
    },

    Move {
        dst: u8,
        src: u8,
    },

    Add {
        dst: u8,
        a: u8,
        b: u8,
    },

    Sub {
        dst: u8,
        a: u8,
        b: u8,
    },

    IfEq {
        a: u8,
        b: u8,
        target: i16,
    },

    Return {
        value: Option<u8>,
    },

    ReturnVoid,

    Print {
        src: u8,
    },

    Unknown(u16),
}