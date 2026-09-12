use crate::runtime::method::Method;

pub struct Frame {
    pub method: Method,
    pub ip: usize,
    pub regs: Vec<i32>,
}

impl Frame {
    pub fn new(method: Method) -> Self {
        Self {
            method,
            ip: 0,
            regs: vec![0; 8],
        }
    }
}