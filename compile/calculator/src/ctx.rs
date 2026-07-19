#[derive(Debug, Clone, Default)]
pub struct Context {
    pub is_loop: bool,
    pub depth: usize,
}
