use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bl_compiler::chunk::Constant;
use bl_compiler::opcode::OpCode;
use bl_compiler::CompiledFunction;
use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::span::Span;
use bl_core::value::Value;

use crate::builtins::BuiltinRegistry;
use crate::frame::{CallFrame, ObjClosure, ObjUpvalue, TryHandler};
use crate::value_ops;

/// Iterator types for the VM's separate iterator stack.
#[derive(Debug)]
pub enum VmIterator {
    List {
        items: Vec<Value>,
        index: usize,
    },
    Range {
        current: i64,
        end: i64,
        inclusive: bool,
    },
    Map {
        pairs: Vec<(String, Value)>,
        index: usize,
    },
    Set {
        items: Vec<Value>,
        index: usize,
    },
}

impl VmIterator {
    pub fn next(&mut self) -> Option<Value> {
        match self {
            VmIterator::List { items, index } => {
                if *index < items.len() {
                    let val = items[*index].clone();
                    *index += 1;
                    Some(val)
                } else {
                    None
                }
            }
            VmIterator::Range {
                current,
                end,
                inclusive,
            } => {
                let in_bounds = if *inclusive {
                    *current <= *end
                } else {
                    *current < *end
                };
                if in_bounds {
                    let val = Value::Int(*current);
                    *current += 1;
                    Some(val)
                } else {
                    None
                }
            }
            VmIterator::Map { pairs, index } => {
                if *index < pairs.len() {
                    let (k, v) = &pairs[*index];
                    let mut rec = HashMap::new();
                    rec.insert("key".to_string(), Value::Str(k.clone()));
                    rec.insert("value".to_string(), v.clone());
                    *index += 1;
                    Some(Value::Record(rec))
                } else {
                    None
                }
            }
            VmIterator::Set { items, index } => {
                if *index < items.len() {
                    let val = items[*index].clone();
                    *index += 1;
                    Some(val)
                } else {
                    None
                }
            }
        }
    }
}

/// The bytecode virtual machine.
pub struct Vm {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    globals: HashMap<String, Value>,
    builtins: BuiltinRegistry,
    iterators: Vec<VmIterator>,
    try_handlers: Vec<TryHandler>,
    pub output_buffer: Option<Arc<Mutex<String>>>,
}

impl Vm {
    pub fn new(builtins: BuiltinRegistry) -> Self {
        Self {
            stack: Vec::with_capacity(256),
            frames: Vec::with_capacity(64),
            globals: HashMap::new(),
            builtins,
            iterators: Vec::new(),
            try_handlers: Vec::new(),
            output_buffer: None,
        }
    }

    /// Set a global variable (used to inject builtins as globals).
    pub fn define_global(&mut self, name: String, value: Value) {
        self.globals.insert(name, value);
    }

    /// Execute a compiled top-level function.
    pub fn execute(&mut self, func: CompiledFunction) -> Result<Value> {
        let closure = ObjClosure {
            function: Arc::new(func),
            upvalues: Vec::new(),
        };
        let frame = CallFrame::new(closure, 0);
        self.frames.push(frame);
        self.run()
    }

    /// Execute a closure (for calling CompiledClosure values from the interpreter).
    pub fn execute_closure(&mut self, closure: ObjClosure, args: Vec<Value>) -> Result<Value> {
        let base = self.stack.len();
        // Push args as locals
        for arg in args {
            self.stack.push(arg);
        }
        let frame = CallFrame::new(closure, base);
        self.frames.push(frame);
        self.run()
    }

    // ── Main dispatch loop ──

    fn run(&mut self) -> Result<Value> {
        loop {
            let frame_idx = self.frames.len() - 1;
            let ip = self.frames[frame_idx].ip;
            let code_len = self.frames[frame_idx].closure.function.chunk.code.len();

            if ip >= code_len {
                // Implicit return
                return Ok(self.stack.pop().unwrap_or(Value::Nil));
            }

            let op = self.frames[frame_idx].closure.function.chunk.code[ip].clone();
            self.frames[frame_idx].ip += 1;

            let result = self.execute_op(op, frame_idx);

            match result {
                Ok(Some(return_val)) => return Ok(return_val),
                Ok(None) => continue,
                Err(e) => {
                    // Check try handlers
                    if let Some(handler) = self.try_handlers.pop() {
                        // Unwind to handler
                        self.stack.truncate(handler.stack_depth);
                        // Push error message as value
                        self.stack.push(Value::Str(e.message.clone()));
                        // Jump to catch handler
                        while self.frames.len() > handler.frame_index + 1 {
                            self.frames.pop();
                        }
                        self.frames[handler.frame_index].ip = handler.catch_ip;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    fn execute_op(
        &mut self,
        op: OpCode,
        frame_idx: usize,
    ) -> Result<Option<Value>> {
        let span = self.current_span(frame_idx);

        match op {
            OpCode::Constant(idx) => {
                let val = self.read_constant(frame_idx, idx);
                self.stack.push(val);
            }
            OpCode::Nil => self.stack.push(Value::Nil),
            OpCode::True => self.stack.push(Value::Bool(true)),
            OpCode::False => self.stack.push(Value::Bool(false)),

            OpCode::Pop => {
                self.stack.pop();
            }
            OpCode::Dup => {
                if let Some(val) = self.stack.last().cloned() {
                    self.stack.push(val);
                }
            }

            OpCode::GetLocal(slot) => {
                let base = self.frames[frame_idx].base;
                let val = self.stack[base + slot as usize].clone();
                self.stack.push(val);
            }
            OpCode::SetLocal(slot) => {
                let base = self.frames[frame_idx].base;
                let val = self.stack.last().cloned().unwrap_or(Value::Nil);
                self.stack[base + slot as usize] = val;
            }

            OpCode::DefineGlobal(idx) => {
                let name = self.read_name(frame_idx, idx);
                let val = self.stack.pop().unwrap_or(Value::Nil);
                self.globals.insert(name, val);
            }
            OpCode::GetGlobal(idx) => {
                let name = self.read_name(frame_idx, idx);
                let val = self.globals.get(&name).cloned().ok_or_else(|| {
                    BioLangError::name_error(format!("undefined variable '{name}'"), span)
                })?;
                self.stack.push(val);
            }
            OpCode::SetGlobal(idx) => {
                let name = self.read_name(frame_idx, idx);
                let val = self.stack.last().cloned().unwrap_or(Value::Nil);
                if self.globals.contains_key(&name) {
                    self.globals.insert(name, val);
                } else {
                    return Err(BioLangError::name_error(
                        format!("undefined variable '{name}'"),
                        span,
                    ));
                }
            }

            OpCode::GetUpvalue(slot) => {
                let val = match &self.frames[frame_idx].closure.upvalues[slot as usize] {
                    ObjUpvalue::Open { stack_index } => self.stack[*stack_index].clone(),
                    ObjUpvalue::Closed(v) => v.clone(),
                };
                self.stack.push(val);
            }
            OpCode::SetUpvalue(slot) => {
                let val = self.stack.last().cloned().unwrap_or(Value::Nil);
                match &mut self.frames[frame_idx].closure.upvalues[slot as usize] {
                    ObjUpvalue::Open { stack_index } => {
                        self.stack[*stack_index] = val;
                    }
                    ObjUpvalue::Closed(ref mut v) => {
                        *v = val;
                    }
                }
            }

            // ── Arithmetic ──
            OpCode::Add => {
                let b = self.stack.pop().unwrap_or(Value::Nil);
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack.push(value_ops::add(a, b, span)?);
            }
            OpCode::Sub => {
                let b = self.stack.pop().unwrap_or(Value::Nil);
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack.push(value_ops::sub(a, b, span)?);
            }
            OpCode::Mul => {
                let b = self.stack.pop().unwrap_or(Value::Nil);
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack.push(value_ops::mul(a, b, span)?);
            }
            OpCode::Div => {
                let b = self.stack.pop().unwrap_or(Value::Nil);
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack.push(value_ops::div(a, b, span)?);
            }
            OpCode::Mod => {
                let b = self.stack.pop().unwrap_or(Value::Nil);
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack.push(value_ops::modulo(a, b, span)?);
            }
            OpCode::Negate => {
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack.push(value_ops::negate(a, span)?);
            }
            OpCode::Not => {
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack.push(Value::Bool(!a.is_truthy()));
            }

            // ── Comparison ──
            OpCode::Equal => {
                let b = self.stack.pop().unwrap_or(Value::Nil);
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack.push(Value::Bool(a == b));
            }
            OpCode::NotEqual => {
                let b = self.stack.pop().unwrap_or(Value::Nil);
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack.push(Value::Bool(a != b));
            }
            OpCode::Less => {
                let b = self.stack.pop().unwrap_or(Value::Nil);
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack.push(Value::Bool(value_ops::less(&a, &b)?));
            }
            OpCode::Greater => {
                let b = self.stack.pop().unwrap_or(Value::Nil);
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack.push(Value::Bool(value_ops::greater(&a, &b)?));
            }
            OpCode::LessEqual => {
                let b = self.stack.pop().unwrap_or(Value::Nil);
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack
                    .push(Value::Bool(value_ops::less_equal(&a, &b)?));
            }
            OpCode::GreaterEqual => {
                let b = self.stack.pop().unwrap_or(Value::Nil);
                let a = self.stack.pop().unwrap_or(Value::Nil);
                self.stack
                    .push(Value::Bool(value_ops::greater_equal(&a, &b)?));
            }

            // ── Control flow ──
            OpCode::Jump(offset) => {
                let new_ip = (self.frames[frame_idx].ip as i32 + offset as i32) as usize;
                self.frames[frame_idx].ip = new_ip;
            }
            OpCode::JumpIfFalse(offset) => {
                let val = self.stack.pop().unwrap_or(Value::Nil);
                if !val.is_truthy() {
                    let new_ip = (self.frames[frame_idx].ip as i32 + offset as i32) as usize;
                    self.frames[frame_idx].ip = new_ip;
                }
            }
            OpCode::JumpIfTrue(offset) => {
                // Does NOT pop — for short-circuit `||`
                if let Some(val) = self.stack.last() {
                    if val.is_truthy() {
                        let new_ip =
                            (self.frames[frame_idx].ip as i32 + offset as i32) as usize;
                        self.frames[frame_idx].ip = new_ip;
                    }
                }
            }
            OpCode::Loop(offset) => {
                self.frames[frame_idx].ip -= offset as usize + 1;
            }

            // ── Functions ──
            OpCode::Call(arg_count) => {
                let callee_idx = self.stack.len() - 1 - arg_count as usize;
                let callee = self.stack[callee_idx].clone();

                match callee {
                    Value::NativeFunction { ref name, .. } => {
                        let args: Vec<Value> = self
                            .stack
                            .drain(callee_idx + 1..)
                            .collect();
                        self.stack.pop(); // pop callee
                        let result = self.builtins.call_by_name(name, args)?;
                        self.stack.push(result);
                    }
                    Value::CompiledClosure(ref any_closure) => {
                        if let Some(closure) = any_closure.downcast_ref::<ObjClosure>() {
                            let closure = closure.clone();
                            let args: Vec<Value> = self
                                .stack
                                .drain(callee_idx + 1..)
                                .collect();
                            self.stack.pop(); // pop callee
                            let base = self.stack.len();
                            for arg in args {
                                self.stack.push(arg);
                            }
                            self.frames
                                .push(CallFrame::new(closure, base));
                        } else {
                            return Err(BioLangError::type_error(
                                "invalid compiled closure",
                                span,
                            ));
                        }
                    }
                    Value::Function { .. } => {
                        // User function from tree-walking interpreter — cannot call from VM
                        // Fall back to builtin callback for HOFs etc.
                        return Err(BioLangError::type_error(
                            "cannot call tree-walking function from bytecode VM",
                            span,
                        ));
                    }
                    _ => {
                        return Err(BioLangError::type_error(
                            format!("{} is not callable", callee.type_of()),
                            span,
                        ));
                    }
                }
            }
            OpCode::CallNative(builtin_id, arg_count) => {
                let start = self.stack.len() - arg_count as usize;
                let args: Vec<Value> = self.stack.drain(start..).collect();
                let result = self.builtins.call_by_id(builtin_id, args)?;
                self.stack.push(result);
            }
            OpCode::Return => {
                let result = self.stack.pop().unwrap_or(Value::Nil);
                let frame = self.frames.pop().unwrap();
                // Clean up locals
                self.stack.truncate(frame.base);

                if self.frames.is_empty() {
                    return Ok(Some(result));
                }
                self.stack.push(result);
            }
            OpCode::Closure(fn_idx) => {
                let func = match &self.frames[frame_idx].closure.function.chunk.constants
                    [fn_idx as usize]
                {
                    Constant::Function(f) => Arc::new(f.clone()),
                    _ => {
                        return Err(BioLangError::runtime(
                            ErrorKind::TypeError,
                            "closure constant is not a function",
                            span,
                        ))
                    }
                };

                let upvalue_count = func.upvalue_count;
                let mut upvalues = Vec::with_capacity(upvalue_count as usize);

                // Upvalue descriptors are not yet wired; create empty upvalues
                for _ in 0..upvalue_count {
                    upvalues.push(ObjUpvalue::Closed(Value::Nil));
                }

                let closure = ObjClosure {
                    function: func,
                    upvalues,
                };
                self.stack.push(Value::CompiledClosure(Arc::new(closure)));
            }
            OpCode::CloseUpvalue => {
                // Close the topmost open upvalue
                let stack_top = self.stack.len() - 1;
                let value = self.stack[stack_top].clone();

                // Walk all frames' upvalues, closing any that point to stack_top
                for frame in &mut self.frames {
                    for upvalue in &mut frame.closure.upvalues {
                        if let ObjUpvalue::Open { stack_index } = upvalue {
                            if *stack_index == stack_top {
                                *upvalue = ObjUpvalue::Closed(value.clone());
                            }
                        }
                    }
                }
                self.stack.pop();
            }

            // ── Data construction ──
            OpCode::MakeList(count) => {
                let start = self.stack.len() - count as usize;
                let items: Vec<Value> = self.stack.drain(start..).collect();
                self.stack.push(Value::List(items));
            }
            OpCode::MakeRecord(count) => {
                // Stack has alternating name_constant_value, actual_value pairs
                let pair_count = count as usize;
                let start = self.stack.len() - pair_count * 2;
                let pairs: Vec<Value> = self.stack.drain(start..).collect();
                let mut map = HashMap::new();
                for chunk in pairs.chunks(2) {
                    if let [Value::Str(key), value] = chunk {
                        map.insert(key.clone(), value.clone());
                    }
                }
                self.stack.push(Value::Record(map));
            }
            OpCode::MakeSet(count) => {
                let start = self.stack.len() - count as usize;
                let items: Vec<Value> = self.stack.drain(start..).collect();
                // Dedup
                let mut unique = Vec::new();
                for item in items {
                    if !unique.contains(&item) {
                        unique.push(item);
                    }
                }
                self.stack.push(Value::Set(unique));
            }
            OpCode::MakeRange(inclusive) => {
                let end = self.stack.pop().unwrap_or(Value::Nil);
                let start = self.stack.pop().unwrap_or(Value::Nil);
                match (&start, &end) {
                    (Value::Int(s), Value::Int(e)) => {
                        self.stack.push(Value::Range {
                            start: *s,
                            end: *e,
                            inclusive: inclusive == 1,
                        });
                    }
                    _ => {
                        return Err(BioLangError::type_error(
                            "range bounds must be integers",
                            span,
                        ))
                    }
                }
            }

            // ── Field / Index access ──
            OpCode::GetField(idx) => {
                let field = self.read_name(frame_idx, idx);
                let object = self.stack.pop().unwrap_or(Value::Nil);
                self.stack
                    .push(value_ops::get_field(&object, &field, span)?);
            }
            OpCode::SetField(idx) => {
                let field = self.read_name(frame_idx, idx);
                let value = self.stack.pop().unwrap_or(Value::Nil);
                let mut object = self.stack.pop().unwrap_or(Value::Nil);
                value_ops::set_field(&mut object, &field, value, span)?;
                self.stack.push(object);
            }
            OpCode::GetFieldOpt(idx) => {
                let field = self.read_name(frame_idx, idx);
                let object = self.stack.pop().unwrap_or(Value::Nil);
                self.stack.push(value_ops::get_field_opt(&object, &field));
            }
            OpCode::GetIndex => {
                let index = self.stack.pop().unwrap_or(Value::Nil);
                let object = self.stack.pop().unwrap_or(Value::Nil);
                self.stack
                    .push(value_ops::get_index(&object, &index, span)?);
            }
            OpCode::SetIndex => {
                let value = self.stack.pop().unwrap_or(Value::Nil);
                let index = self.stack.pop().unwrap_or(Value::Nil);
                let object = self.stack.pop().unwrap_or(Value::Nil);
                // Simplified: only records/maps
                match (object, &index) {
                    (Value::Record(mut map), Value::Str(key))
                    | (Value::Map(mut map), Value::Str(key)) => {
                        map.insert(key.clone(), value);
                        self.stack.push(Value::Record(map));
                    }
                    (Value::List(mut list), Value::Int(i)) => {
                        let idx = if *i < 0 {
                            (list.len() as i64 + i) as usize
                        } else {
                            *i as usize
                        };
                        if idx < list.len() {
                            list[idx] = value;
                        }
                        self.stack.push(Value::List(list));
                    }
                    (obj, _) => {
                        return Err(BioLangError::type_error(
                            format!("cannot set index on {}", obj.type_of()),
                            span,
                        ))
                    }
                }
            }

            // ── Iteration ──
            OpCode::PushIter => {
                let val = self.stack.pop().unwrap_or(Value::Nil);
                let iter = match val {
                    Value::List(items) => VmIterator::List { items, index: 0 },
                    Value::Range {
                        start,
                        end,
                        inclusive,
                    } => VmIterator::Range {
                        current: start,
                        end,
                        inclusive,
                    },
                    Value::Map(map) | Value::Record(map) => {
                        let pairs: Vec<(String, Value)> = map.into_iter().collect();
                        VmIterator::Map { pairs, index: 0 }
                    }
                    Value::Set(items) => VmIterator::Set { items, index: 0 },
                    Value::Stream(s) => {
                        // Collect stream into list iterator
                        let items = s.collect_all();
                        VmIterator::List { items, index: 0 }
                    }
                    Value::Str(s) => {
                        let items: Vec<Value> =
                            s.chars().map(|c| Value::Str(c.to_string())).collect();
                        VmIterator::List { items, index: 0 }
                    }
                    other => {
                        return Err(BioLangError::type_error(
                            format!("cannot iterate over {}", other.type_of()),
                            span,
                        ))
                    }
                };
                self.iterators.push(iter);
            }
            OpCode::IterNext(offset) => {
                if let Some(iter) = self.iterators.last_mut() {
                    match iter.next() {
                        Some(val) => {
                            self.stack.push(val);
                        }
                        None => {
                            // Jump to exit
                            let new_ip = (self.frames[frame_idx].ip as i32
                                + offset as i32) as usize;
                            self.frames[frame_idx].ip = new_ip;
                        }
                    }
                }
            }
            OpCode::PopIter => {
                self.iterators.pop();
            }

            // ── String interpolation ──
            OpCode::StringInterp(count) => {
                let start = self.stack.len() - count as usize;
                let parts: Vec<Value> = self.stack.drain(start..).collect();
                let result: String = parts.iter().map(|v| format!("{v}")).collect();
                self.stack.push(Value::Str(result));
            }

            // ── Bio literals ──
            OpCode::MakeDna(idx) => {
                let seq_str = self.read_name(frame_idx, idx);
                self.stack.push(Value::DNA(bl_core::value::BioSequence {
                    data: seq_str,
                }));
            }
            OpCode::MakeRna(idx) => {
                let seq_str = self.read_name(frame_idx, idx);
                self.stack.push(Value::RNA(bl_core::value::BioSequence {
                    data: seq_str,
                }));
            }
            OpCode::MakeProtein(idx) => {
                let seq_str = self.read_name(frame_idx, idx);
                self.stack
                    .push(Value::Protein(bl_core::value::BioSequence { data: seq_str }));
            }

            // ── Exception handling ──
            OpCode::TryBegin(catch_offset) => {
                let catch_ip = self.frames[frame_idx].ip + catch_offset as usize;
                self.try_handlers.push(TryHandler {
                    frame_index: frame_idx,
                    catch_ip,
                    stack_depth: self.stack.len(),
                });
            }
            OpCode::TryEnd => {
                self.try_handlers.pop();
            }
            OpCode::Throw => {
                let val = self.stack.pop().unwrap_or(Value::Nil);
                let msg = format!("{val}");
                return Err(BioLangError::runtime(ErrorKind::TypeError, msg, span));
            }

            // ── Null coalescing ──
            OpCode::NullCoalesce(offset) => {
                if let Some(val) = self.stack.last() {
                    if !matches!(val, Value::Nil) {
                        // Non-nil: skip right side
                        let new_ip =
                            (self.frames[frame_idx].ip as i32 + offset as i32) as usize;
                        self.frames[frame_idx].ip = new_ip;
                    }
                }
            }

            // ── Special ──
            OpCode::MakeFormula(idx) => {
                match &self.frames[frame_idx].closure.function.chunk.constants[idx as usize] {
                    Constant::AstFragment(ast) => {
                        self.stack.push(Value::Formula(Box::new(ast.as_ref().clone())));
                    }
                    _ => self.stack.push(Value::Nil),
                }
            }
            OpCode::Import(_, _) => {
                // Import is handled at runtime level
                self.stack.push(Value::Nil);
            }
            OpCode::AssertCheck => {
                let msg = self.stack.pop().unwrap_or(Value::Nil);
                let cond = self.stack.pop().unwrap_or(Value::Nil);
                if !cond.is_truthy() {
                    let message = if matches!(msg, Value::Nil) {
                        format!("assertion failed: expression evaluated to {cond}")
                    } else {
                        format!("{msg}")
                    };
                    return Err(BioLangError::runtime(
                        ErrorKind::AssertionFailed,
                        message,
                        span,
                    ));
                }
            }

            OpCode::DebugSpan(_, _) => {
                // No-op: span info is for debugging tools
            }
        }

        Ok(None)
    }

    // ── Helpers ──

    fn current_span(&self, frame_idx: usize) -> Option<Span> {
        let ip = self.frames[frame_idx].ip.saturating_sub(1);
        self.frames[frame_idx]
            .closure
            .function
            .chunk
            .span_at(ip)
    }

    fn read_constant(&self, frame_idx: usize, idx: u16) -> Value {
        match &self.frames[frame_idx].closure.function.chunk.constants[idx as usize] {
            Constant::Value(v) => v.clone(),
            Constant::Name(n) => Value::Str(n.clone()),
            Constant::Function(_) => Value::Nil, // functions are accessed via Closure opcode
            Constant::AstFragment(_) => Value::Nil,
        }
    }

    fn read_name(&self, frame_idx: usize, idx: u16) -> String {
        match &self.frames[frame_idx].closure.function.chunk.constants[idx as usize] {
            Constant::Name(n) => n.clone(),
            Constant::Value(Value::Str(s)) => s.clone(),
            _ => format!("#{idx}"),
        }
    }
}
