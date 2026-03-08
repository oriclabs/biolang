/// Describes how an upvalue is captured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UpvalueDescriptor {
    /// If true, the upvalue captures a local from the immediately enclosing function.
    /// If false, it captures an upvalue from the enclosing function.
    pub is_local: bool,
    /// Index of the local slot (if is_local) or upvalue slot (if !is_local)
    /// in the enclosing function.
    pub index: u16,
}
