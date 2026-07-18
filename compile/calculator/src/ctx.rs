#[derive(Debug, Clone)]
pub struct Context {
    pub is_loop: bool,
    pub depth: usize,
}

impl Default for Context {
    fn default() -> Self {
        Context {
            is_loop: false,
            depth: 0,
        }
    }
}
