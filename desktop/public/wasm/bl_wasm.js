/**
 * A genuinely isolated browser interpreter owned by one JavaScript SDK
 * session. Module-level functions remain available for compatibility, but
 * new SDK sessions use this class so variables and resets never cross session
 * boundaries within the same worker or Node process.
 */
export class WasmSession {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmSessionFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmsession_free(ptr, 0);
    }
    /**
     * Call a BioLang function with JavaScript values without constructing or
     * parsing BioLang source.
     * @param {string} name
     * @param {any} _arguments
     * @param {number} maximum_inline_bytes
     * @returns {any}
     */
    call_value(name, _arguments, maximum_inline_bytes) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsession_call_value(this.__wbg_ptr, ptr0, len0, _arguments, maximum_inline_bytes);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Call a BioLang function with separate positional and named JavaScript
     * values. This keeps notebook JavaScript readable without serializing a
     * call back into BioLang source.
     * @param {string} name
     * @param {any} positional
     * @param {any} named
     * @param {number} maximum_inline_bytes
     * @returns {any}
     */
    call_value_named(name, positional, named, maximum_inline_bytes) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsession_call_value_named(this.__wbg_ptr, ptr0, len0, positional, named, maximum_inline_bytes);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Evaluate source and return the actual BioLang value as JavaScript data.
     * Values above `maximum_inline_bytes` stay in Rust and return a handle.
     * @param {string} source
     * @param {number} maximum_inline_bytes
     * @returns {any}
     */
    eval_value(source, maximum_inline_bytes) {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsession_eval_value(this.__wbg_ptr, ptr0, len0, maximum_inline_bytes);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {string} source
     * @returns {string}
     */
    evaluate(source) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmsession_evaluate(this.__wbg_ptr, ptr0, len0);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @param {string} name
     * @param {string} format
     * @param {number} maximum_bytes
     * @returns {Uint8Array}
     */
    export_variable(name, format, maximum_bytes) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsession_export_variable(this.__wbg_ptr, ptr0, len0, ptr1, len1, maximum_bytes);
        if (ret[3]) {
            throw takeFromExternrefTable0(ret[2]);
        }
        var v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v3;
    }
    /**
     * Read a field without copying an opaque value out of its owning session.
     * @param {any} value
     * @param {string} field
     * @param {boolean} optional
     * @param {number} maximum_inline_bytes
     * @returns {any}
     */
    field_value(value, field, optional, maximum_inline_bytes) {
        const ptr0 = passStringToWasm0(field, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsession_field_value(this.__wbg_ptr, value, ptr0, len0, optional, maximum_inline_bytes);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Format one JavaScript value with BioLang's f-string mini-language.
     * @param {any} value
     * @param {string} spec
     * @returns {string}
     */
    format_value(value, spec) {
        let deferred3_0;
        let deferred3_1;
        try {
            const ptr0 = passStringToWasm0(spec, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmsession_format_value(this.__wbg_ptr, value, ptr0, len0);
            var ptr2 = ret[0];
            var len2 = ret[1];
            if (ret[3]) {
                ptr2 = 0; len2 = 0;
                throw takeFromExternrefTable0(ret[2]);
            }
            deferred3_0 = ptr2;
            deferred3_1 = len2;
            return getStringFromWasm0(ptr2, len2);
        } finally {
            wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
        }
    }
    /**
     * Read a named BioLang variable as JavaScript data or a session handle.
     * @param {string} name
     * @param {number} maximum_inline_bytes
     * @returns {any}
     */
    get_value(name, maximum_inline_bytes) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsession_get_value(this.__wbg_ptr, ptr0, len0, maximum_inline_bytes);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @returns {number}
     */
    handle_count() {
        const ret = wasm.wasmsession_handle_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @param {number} id
     * @param {number} generation
     * @returns {Float64Array}
     */
    handle_float64(id, generation) {
        const ret = wasm.wasmsession_handle_float64(this.__wbg_ptr, id, generation);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {number} id
     * @param {number} generation
     * @param {number} offset
     * @param {number} limit
     * @returns {any}
     */
    handle_page(id, generation, offset, limit) {
        const ret = wasm.wasmsession_handle_page(this.__wbg_ptr, id, generation, offset, limit);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * @param {string} name
     * @param {number} offset
     * @param {number} limit
     * @returns {string}
     */
    inspect_variable(name, offset, limit) {
        let deferred2_0;
        let deferred2_1;
        try {
            const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len0 = WASM_VECTOR_LEN;
            const ret = wasm.wasmsession_inspect_variable(this.__wbg_ptr, ptr0, len0, offset, limit);
            deferred2_0 = ret[0];
            deferred2_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
        }
    }
    /**
     * @returns {string}
     */
    list_variables() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmsession_list_variables(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    constructor() {
        const ret = wasm.wasmsession_new();
        this.__wbg_ptr = ret;
        WasmSessionFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Register a synchronous JavaScript callback as a callable BioLang native
     * function in this interpreter only.
     * @param {string} name
     * @param {number} minimum_arguments
     * @param {number} maximum_arguments
     */
    register_host_function(name, minimum_arguments, maximum_arguments) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsession_register_host_function(this.__wbg_ptr, ptr0, len0, minimum_arguments, maximum_arguments);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * @param {string} path
     * @param {string} source
     */
    register_module(path, source) {
        const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        wasm.wasmsession_register_module(this.__wbg_ptr, ptr0, len0, ptr1, len1);
    }
    /**
     * @param {number} id
     * @param {number} generation
     * @returns {boolean}
     */
    release_handle(id, generation) {
        const ret = wasm.wasmsession_release_handle(this.__wbg_ptr, id, generation);
        return ret !== 0;
    }
    reset() {
        wasm.wasmsession_reset(this.__wbg_ptr);
    }
    /**
     * Define a BioLang variable directly from JavaScript data.
     * @param {string} name
     * @param {any} value
     */
    set_value(name, value) {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmsession_set_value(this.__wbg_ptr, ptr0, len0, value);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
}
if (Symbol.dispose) WasmSession.prototype[Symbol.dispose] = WasmSession.prototype.free;

/**
 * Evaluate BioLang source code in the legacy module-level interpreter.
 *
 * This state is shared by every caller of the module-level API. Concurrent or
 * security-sensitive embedders should construct `WasmSession` instead.
 * Returns JSON: `{ok, value, type, output, error}`.
 *
 * @deprecated Use `WasmSession.evaluate` for isolated state.
 * @param {string} source
 * @returns {string}
 */
export function evaluate(source) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.evaluate(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Serialize a variable from the shared legacy module-level interpreter,
 * stopping before the response can exceed the caller's byte cap. Large native
 * exports use the streaming path in `bl_runtime::value_export` instead of
 * crossing this in-memory boundary.
 *
 * @deprecated Use `WasmSession.export_variable` for isolated state.
 * @param {string} name
 * @param {string} format
 * @param {number} maximum_bytes
 * @returns {Uint8Array}
 */
export function export_variable(name, format, maximum_bytes) {
    const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.export_variable(ptr0, len0, ptr1, len1, maximum_bytes);
    if (ret[3]) {
        throw takeFromExternrefTable0(ret[2]);
    }
    var v3 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
    return v3;
}

/**
 * Format BioLang source into the canonical layout.
 *
 * The browser build has no language server, so without this the web workbench
 * would be the one place `bl fmt` cannot reach — and a formatter people cannot
 * rely on everywhere is one they stop running.
 * @param {string} source
 * @param {number} indent
 * @returns {string}
 */
export function format(source, indent) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.format(ptr0, len0, indent);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Convert Python, R, Jupyter, or R Markdown and return a structured validation result.
 * @param {string} source
 * @param {string} format
 * @param {string} filename
 * @returns {string}
 */
export function import_source(source, format, filename) {
    let deferred4_0;
    let deferred4_1;
    try {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(format, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ptr2 = passStringToWasm0(filename, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len2 = WASM_VECTOR_LEN;
        const ret = wasm.import_source(ptr0, len0, ptr1, len1, ptr2, len2);
        deferred4_0 = ret[0];
        deferred4_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred4_0, deferred4_1, 1);
    }
}

/**
 * Initialize the WASM module (set panic hook for better error messages).
 */
export function init() {
    wasm.init();
}

/**
 * Return one bounded page from the shared legacy module-level interpreter.
 * Container values are never formatted wholesale: at most 100 rows and 50
 * columns cross the WASM boundary per call.
 *
 * @deprecated Use `WasmSession.inspect_variable` for isolated state.
 * @param {string} name
 * @param {number} offset
 * @param {number} limit
 * @returns {string}
 */
export function inspect_variable(name, offset, limit) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.inspect_variable(ptr0, len0, offset, limit);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Return builtin and keyword completions matching a prefix.
 * @param {string} prefix
 * @returns {string}
 */
export function language_completions(prefix) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(prefix, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.language_completions(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Return parser diagnostics with byte offsets and line/column locations.
 * This is intentionally filesystem-free so browser and native editors agree.
 * @param {string} source
 * @returns {string}
 */
export function language_diagnostics(source) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.language_diagnostics(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Return a compact builtin signature, or JSON null when the name is unknown.
 * @param {string} name
 * @returns {string}
 */
export function language_signature(name) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.language_signature(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * List all builtin functions available in this WASM build.
 * Returns a JSON array of `{name, arity}` records.
 * @returns {string}
 */
export function list_builtins() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.list_builtins();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * List variables in the shared legacy module-level interpreter.
 * Returns a JSON array.
 *
 * @deprecated Use `WasmSession.list_variables` for isolated state.
 * @returns {string}
 */
export function list_variables() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.list_variables();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Quality metrics for a sequencing file preview, as JSON.
 *
 * Returns `null` when the format has no metrics or the sample is unusable.
 * Shared with the Desktop build through `bl-qc` so both report the same
 * numbers rather than two implementations quietly disagreeing.
 * @param {string} kind
 * @param {string} text
 * @returns {string}
 */
export function qc_metrics(kind, text) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(kind, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(text, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.qc_metrics(ptr0, len0, ptr1, len1);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Register an optional in-memory package in the shared legacy interpreter.
 *
 * Modules supplied through this function remain outside the BioLang WASM
 * artifact. Isolated applications should call `WasmSession.register_module`.
 * The default browser interpreter separately embeds the small core statistics
 * modules that are available without registration.
 *
 * @deprecated Use `WasmSession.register_module` for isolated state.
 * @param {string} path
 * @param {string} source
 */
export function register_module(path, source) {
    const ptr0 = passStringToWasm0(path, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    wasm.register_module(ptr0, len0, ptr1, len1);
}

/**
 * Reset the shared legacy module-level interpreter state.
 *
 * @deprecated Use `WasmSession.reset` for isolated state.
 */
export function reset() {
    wasm.reset();
}

/**
 * Version of the Rust runtime compiled into this WebAssembly module.
 *
 * JavaScript packages must use this value for compatibility checks rather
 * than reporting their independently versioned package.json version.
 * @returns {string}
 */
export function runtime_version() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.runtime_version();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}

/**
 * Tokenize source code for syntax highlighting. Returns JSON array of token spans.
 * @param {string} source
 * @returns {string}
 */
export function tokenize(source) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.tokenize(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Convert BioLang source into readable direct JavaScript SDK calls.
 *
 * Ordinary code uses native JavaScript syntax and direct `bl.*` calls. Rare
 * declarations without a natural JavaScript form use structural SDK helpers.
 * The returned JavaScript never embeds or evaluates a BioLang source string.
 * @param {string} source
 * @returns {string}
 */
export function transpile_javascript(source) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.transpile_javascript(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Validate a BioLang script or BioLang notebook without executing it.
 * @param {string} source
 * @param {boolean} notebook
 * @returns {string}
 */
export function validate_import(source, notebook) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.validate_import(ptr0, len0, notebook);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    } finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_boolean_get_fa956cfa2d1bd751: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_is_null_ea9085d691f535d3: function(arg0) {
            const ret = arg0 === null;
            return ret;
        },
        __wbg___wbindgen_is_object_a27215656b807791: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_undefined_c05833b95a3cf397: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_number_get_394265ed1e1b84ee: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_b0ca35b86a603356: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_344f42d3211c4765: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_call_159eb9590236ae2b: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = window.__blCallbacks.call(getStringFromWasm0(arg0, arg1), arg2);
            return ret;
        }, arguments); },
        __wbg_create_8dbb796b6159a8e4: function(arg0) {
            const ret = Object.create(arg0);
            return ret;
        },
        __wbg_error_a6fa202b58aa1cd3: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_from_13e323c65fc8f464: function(arg0) {
            const ret = Array.from(arg0);
            return ret;
        },
        __wbg_getRandomValues_cc7f052a444bb2ce: function() { return handleError(function (arg0, arg1) {
            globalThis.crypto.getRandomValues(getArrayU8FromWasm0(arg0, arg1));
        }, arguments); },
        __wbg_getTime_d6f070c088c9b5ed: function(arg0) {
            const ret = arg0.getTime();
            return ret;
        },
        __wbg_get_507a50627bffa49b: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_78f252d074a84d0b: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_instanceof_Float64Array_92032ec8f216bceb: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Float64Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Uint8Array_309b927aaf7a3fc7: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_0677c962b281d01a: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_keys_58421f8f96795607: function(arg0) {
            const ret = Object.keys(arg0);
            return ret;
        },
        __wbg_length_0133fa10e3234c57: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_1f0964f4a5e2c6d8: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_370319915dc99107: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_new_0_3da9e97f24fc69be: function() {
            const ret = new Date();
            return ret;
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_new_80efd62ef570c5de: function(arg0) {
            const ret = new Float64Array(arg0);
            return ret;
        },
        __wbg_new_cd45aabdf6073e84: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_da52cf8fe3429cb2: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_from_slice_77cdfb7977362f3c: function(arg0, arg1) {
            const ret = new Uint8Array(getArrayU8FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_from_slice_7e254b47c77fb8cc: function(arg0, arg1) {
            const ret = new Float64Array(getArrayF64FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_with_length_f8cbc3a5b9ff9368: function(arg0) {
            const ret = new Array(arg0 >>> 0);
            return ret;
        },
        __wbg_prototypesetcall_21a175a0a8157491: function(arg0, arg1, arg2) {
            Float64Array.prototype.set.call(getArrayF64FromWasm0(arg0, arg1), arg2);
        },
        __wbg_prototypesetcall_4770620bbe4688a0: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_set_8535240470bf2500: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_8a16b38e4805b298: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_sync_8070627e5cb4dd85: function(arg0, arg1) {
            const ret = window.__blFetch.sync(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbindgen_cast_0000000000000001: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./bl_wasm_bg.js": import0,
    };
}

const WasmSessionFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmsession_free(ptr, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function getArrayF64FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat64ArrayMemory0 = null;
function getFloat64ArrayMemory0() {
    if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
        cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachedFloat64ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat64ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('bl_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
