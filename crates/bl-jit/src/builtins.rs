use bl_core::error::Result;
use bl_core::value::{Arity, Value};

/// Callback trait that decouples the VM from the runtime's builtin implementations.
/// The consuming crate (bl-runtime) provides the implementation.
pub trait BuiltinCallback: Send + Sync {
    /// Call a builtin function by name with the given arguments.
    fn call_builtin(&self, name: &str, args: Vec<Value>) -> Result<Value>;

    /// Return the list of all builtin names and their arities.
    fn builtin_list(&self) -> Vec<(String, Arity)>;
}

/// Registry of builtin functions for the VM.
pub struct BuiltinRegistry {
    callback: Box<dyn BuiltinCallback>,
    /// Ordered list of builtin names (index = builtin ID for CallNative).
    names: Vec<String>,
}

impl BuiltinRegistry {
    pub fn new(callback: Box<dyn BuiltinCallback>) -> Self {
        let list = callback.builtin_list();
        let names: Vec<String> = list.into_iter().map(|(name, _)| name).collect();
        Self { callback, names }
    }

    /// Look up a builtin name by its numeric ID.
    pub fn name_by_id(&self, id: u16) -> Option<&str> {
        self.names.get(id as usize).map(|s| s.as_str())
    }

    /// Look up a builtin ID by name.
    pub fn id_by_name(&self, name: &str) -> Option<u16> {
        self.names.iter().position(|n| n == name).map(|i| i as u16)
    }

    /// Call a builtin by its numeric ID.
    pub fn call_by_id(&self, id: u16, args: Vec<Value>) -> Result<Value> {
        if let Some(name) = self.name_by_id(id) {
            self.callback.call_builtin(name, args)
        } else {
            Err(bl_core::error::BioLangError::runtime(
                bl_core::error::ErrorKind::NameError,
                format!("unknown builtin id: {id}"),
                None,
            ))
        }
    }

    /// Call a builtin by name.
    pub fn call_by_name(&self, name: &str, args: Vec<Value>) -> Result<Value> {
        self.callback.call_builtin(name, args)
    }

    /// Get the name-to-id mappings for the compiler.
    pub fn name_id_pairs(&self) -> Vec<(String, u16)> {
        self.names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u16))
            .collect()
    }
}
