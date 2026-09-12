// vm/frame.rs

#[derive(Debug)]
pub struct Frame {
    pub locals: Vec<i32>,
}

impl Frame {
    pub fn new(size: usize) -> Self {
        Self {
            locals: vec![0; size],
        }
    }
}