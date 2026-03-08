use crate::chunk::{Chunk, CompiledFunction, Constant};
use crate::opcode::OpCode;
use bl_core::value::Value;

/// Disassemble a compiled function into a human-readable string.
pub fn disassemble_function(func: &CompiledFunction) -> String {
    let mut out = String::new();
    let name = func.name.as_deref().unwrap_or("<script>");
    out.push_str(&format!(
        "== {} (arity={}, upvalues={}) ==\n",
        name, func.arity, func.upvalue_count
    ));
    disassemble_chunk(&func.chunk, &mut out);

    // Recursively disassemble nested functions
    for constant in &func.chunk.constants {
        if let Constant::Function(nested) = constant {
            out.push('\n');
            out.push_str(&disassemble_function(nested));
        }
    }
    out
}

/// Disassemble a chunk into a human-readable listing.
pub fn disassemble_chunk(chunk: &Chunk, out: &mut String) {
    for (i, op) in chunk.code.iter().enumerate() {
        disassemble_instruction(chunk, i, op, out);
    }
}

fn disassemble_instruction(chunk: &Chunk, offset: usize, op: &OpCode, out: &mut String) {
    out.push_str(&format!("{:04} ", offset));

    // Show span info if available
    if let Some(span) = chunk.span_at(offset) {
        out.push_str(&format!("[{:>4}:{:<4}] ", span.start, span.end));
    } else {
        out.push_str("           ");
    }

    match op {
        OpCode::Constant(idx) => {
            let val = constant_display(chunk, *idx);
            out.push_str(&format!("{:<20} {:>5} ({})\n", "CONSTANT", idx, val));
        }
        OpCode::Nil => out.push_str("NIL\n"),
        OpCode::True => out.push_str("TRUE\n"),
        OpCode::False => out.push_str("FALSE\n"),
        OpCode::Pop => out.push_str("POP\n"),
        OpCode::Dup => out.push_str("DUP\n"),
        OpCode::GetLocal(slot) => {
            out.push_str(&format!("{:<20} {:>5}\n", "GET_LOCAL", slot));
        }
        OpCode::SetLocal(slot) => {
            out.push_str(&format!("{:<20} {:>5}\n", "SET_LOCAL", slot));
        }
        OpCode::DefineGlobal(idx) => {
            let name = name_display(chunk, *idx);
            out.push_str(&format!("{:<20} {:>5} ({})\n", "DEFINE_GLOBAL", idx, name));
        }
        OpCode::GetGlobal(idx) => {
            let name = name_display(chunk, *idx);
            out.push_str(&format!("{:<20} {:>5} ({})\n", "GET_GLOBAL", idx, name));
        }
        OpCode::SetGlobal(idx) => {
            let name = name_display(chunk, *idx);
            out.push_str(&format!("{:<20} {:>5} ({})\n", "SET_GLOBAL", idx, name));
        }
        OpCode::GetUpvalue(idx) => {
            out.push_str(&format!("{:<20} {:>5}\n", "GET_UPVALUE", idx));
        }
        OpCode::SetUpvalue(idx) => {
            out.push_str(&format!("{:<20} {:>5}\n", "SET_UPVALUE", idx));
        }
        OpCode::Add => out.push_str("ADD\n"),
        OpCode::Sub => out.push_str("SUB\n"),
        OpCode::Mul => out.push_str("MUL\n"),
        OpCode::Div => out.push_str("DIV\n"),
        OpCode::Mod => out.push_str("MOD\n"),
        OpCode::Negate => out.push_str("NEGATE\n"),
        OpCode::Not => out.push_str("NOT\n"),
        OpCode::Equal => out.push_str("EQUAL\n"),
        OpCode::NotEqual => out.push_str("NOT_EQUAL\n"),
        OpCode::Less => out.push_str("LESS\n"),
        OpCode::Greater => out.push_str("GREATER\n"),
        OpCode::LessEqual => out.push_str("LESS_EQUAL\n"),
        OpCode::GreaterEqual => out.push_str("GREATER_EQUAL\n"),
        OpCode::Jump(offset) => {
            out.push_str(&format!("{:<20} {:>5}\n", "JUMP", offset));
        }
        OpCode::JumpIfFalse(offset) => {
            out.push_str(&format!("{:<20} {:>5}\n", "JUMP_IF_FALSE", offset));
        }
        OpCode::JumpIfTrue(offset) => {
            out.push_str(&format!("{:<20} {:>5}\n", "JUMP_IF_TRUE", offset));
        }
        OpCode::Loop(offset) => {
            out.push_str(&format!("{:<20} {:>5}\n", "LOOP", offset));
        }
        OpCode::Call(argc) => {
            out.push_str(&format!("{:<20} {:>5}\n", "CALL", argc));
        }
        OpCode::CallNative(id, argc) => {
            out.push_str(&format!("{:<20} id={:<3} argc={}\n", "CALL_NATIVE", id, argc));
        }
        OpCode::Return => out.push_str("RETURN\n"),
        OpCode::Closure(idx) => {
            let name = match chunk.constants.get(*idx as usize) {
                Some(Constant::Function(f)) => {
                    f.name.as_deref().unwrap_or("<closure>").to_string()
                }
                _ => format!("#{}", idx),
            };
            out.push_str(&format!("{:<20} {:>5} ({})\n", "CLOSURE", idx, name));
        }
        OpCode::CloseUpvalue => out.push_str("CLOSE_UPVALUE\n"),
        OpCode::MakeList(n) => {
            out.push_str(&format!("{:<20} {:>5}\n", "MAKE_LIST", n));
        }
        OpCode::MakeRecord(n) => {
            out.push_str(&format!("{:<20} {:>5}\n", "MAKE_RECORD", n));
        }
        OpCode::MakeSet(n) => {
            out.push_str(&format!("{:<20} {:>5}\n", "MAKE_SET", n));
        }
        OpCode::MakeRange(inclusive) => {
            let label = if *inclusive == 1 { "inclusive" } else { "exclusive" };
            out.push_str(&format!("{:<20} ({})\n", "MAKE_RANGE", label));
        }
        OpCode::GetField(idx) => {
            let name = name_display(chunk, *idx);
            out.push_str(&format!("{:<20} {:>5} ({})\n", "GET_FIELD", idx, name));
        }
        OpCode::SetField(idx) => {
            let name = name_display(chunk, *idx);
            out.push_str(&format!("{:<20} {:>5} ({})\n", "SET_FIELD", idx, name));
        }
        OpCode::GetFieldOpt(idx) => {
            let name = name_display(chunk, *idx);
            out.push_str(&format!("{:<20} {:>5} ({})\n", "GET_FIELD_OPT", idx, name));
        }
        OpCode::GetIndex => out.push_str("GET_INDEX\n"),
        OpCode::SetIndex => out.push_str("SET_INDEX\n"),
        OpCode::PushIter => out.push_str("PUSH_ITER\n"),
        OpCode::IterNext(offset) => {
            out.push_str(&format!("{:<20} {:>5}\n", "ITER_NEXT", offset));
        }
        OpCode::PopIter => out.push_str("POP_ITER\n"),
        OpCode::StringInterp(n) => {
            out.push_str(&format!("{:<20} {:>5}\n", "STRING_INTERP", n));
        }
        OpCode::MakeDna(idx) => {
            let name = name_display(chunk, *idx);
            out.push_str(&format!("{:<20} {:>5} ({})\n", "MAKE_DNA", idx, name));
        }
        OpCode::MakeRna(idx) => {
            let name = name_display(chunk, *idx);
            out.push_str(&format!("{:<20} {:>5} ({})\n", "MAKE_RNA", idx, name));
        }
        OpCode::MakeProtein(idx) => {
            let name = name_display(chunk, *idx);
            out.push_str(&format!("{:<20} {:>5} ({})\n", "MAKE_PROTEIN", idx, name));
        }
        OpCode::TryBegin(offset) => {
            out.push_str(&format!("{:<20} {:>5}\n", "TRY_BEGIN", offset));
        }
        OpCode::TryEnd => out.push_str("TRY_END\n"),
        OpCode::Throw => out.push_str("THROW\n"),
        OpCode::NullCoalesce(offset) => {
            out.push_str(&format!("{:<20} {:>5}\n", "NULL_COALESCE", offset));
        }
        OpCode::MakeFormula(idx) => {
            out.push_str(&format!("{:<20} {:>5}\n", "MAKE_FORMULA", idx));
        }
        OpCode::Import(idx, has_alias) => {
            let name = name_display(chunk, *idx);
            out.push_str(&format!(
                "{:<20} {:>5} ({}) alias={}\n",
                "IMPORT", idx, name, has_alias
            ));
        }
        OpCode::AssertCheck => out.push_str("ASSERT_CHECK\n"),
        OpCode::DebugSpan(start, end) => {
            out.push_str(&format!("{:<20} {}..{}\n", "DEBUG_SPAN", start, end));
        }
    }
}

fn constant_display(chunk: &Chunk, idx: u16) -> String {
    match chunk.constants.get(idx as usize) {
        Some(Constant::Value(v)) => format!("{v}"),
        Some(Constant::Name(n)) => format!("'{n}'"),
        Some(Constant::Function(f)) => {
            format!("<fn {}>", f.name.as_deref().unwrap_or("?"))
        }
        Some(Constant::AstFragment(_)) => "<ast>".to_string(),
        None => "<invalid>".to_string(),
    }
}

fn name_display(chunk: &Chunk, idx: u16) -> String {
    match chunk.constants.get(idx as usize) {
        Some(Constant::Name(n)) => n.clone(),
        Some(Constant::Value(Value::Str(s))) => s.clone(),
        _ => format!("#{idx}"),
    }
}
