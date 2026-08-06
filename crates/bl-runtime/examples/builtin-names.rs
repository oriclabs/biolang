//! Print every builtin this configuration registers, one per line, sorted.
//!
//! Exists so a script can ask the source what it defines. Built with
//! `--no-default-features` this is the browser build's registry - the `native`
//! feature is the only thing that separates the two - which is what
//! scripts/check-wasm-fresh.mjs compares the shipped WASM module against.
//!
//!   cargo run -q --example builtin-names -p bl-runtime --no-default-features

fn main() {
    let mut names = bl_runtime::builtins::all_builtin_names();
    names.sort_unstable();
    let mut out = String::new();
    for name in names {
        out.push_str(name);
        out.push('\n');
    }
    print!("{out}");
}
