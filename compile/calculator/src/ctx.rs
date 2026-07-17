#[derive(Debug, Clone)]
pub struct Context {
    pub is_loop: bool,
}

impl Default for Context {
    fn default() -> Self {
        Context { is_loop: false }
    }
}
