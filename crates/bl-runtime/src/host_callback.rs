use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::Value;
use std::cell::RefCell;
use std::sync::Arc;

/// Internal native-function prefix used for host callbacks. User-visible names
/// remain ordinary identifiers in an interpreter environment.
pub const HOST_CALLBACK_PREFIX: &str = "__host_callback:";

type HostCallback = dyn Fn(&str, Vec<Value>) -> Result<Value>;

thread_local! {
    static HOST_CALLBACK: RefCell<Option<Arc<HostCallback>>> = const { RefCell::new(None) };
}

/// Install the callback dispatcher supplied by an embedding runtime.
pub fn set_host_callback_hook(hook: Option<Arc<HostCallback>>) {
    HOST_CALLBACK.with(|cell| *cell.borrow_mut() = hook);
}

/// Invoke a callback through the current embedding runtime.
pub fn call_host_callback(name: &str, args: Vec<Value>) -> Result<Value> {
    HOST_CALLBACK.with(|cell| {
        let hook = cell.borrow().clone().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::PluginError,
                format!("host callback '{name}' is unavailable in this runtime"),
                None,
            )
        })?;
        hook(name, args)
    })
}
