/// Tracks loop state for break/continue jump patching.
#[derive(Debug)]
pub struct LoopContext {
    /// Instruction index of the loop header (target for `continue`).
    pub loop_start: usize,
    /// Instruction indices of `break` jumps that need patching on loop exit.
    pub break_jumps: Vec<usize>,
    /// Scope depth at loop entry (for closing upvalues on break/continue).
    pub scope_depth: u32,
}

impl LoopContext {
    pub fn new(loop_start: usize, scope_depth: u32) -> Self {
        Self {
            loop_start,
            break_jumps: Vec::new(),
            scope_depth,
        }
    }
}
