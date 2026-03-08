use bl_compiler::*;
use bl_core::ast::*;
use bl_core::span::{Span, Spanned};
use bl_core::value::Value;

fn s<T>(node: T) -> Spanned<T> {
    Spanned::new(node, Span::new(0, 0))
}

fn compile(program: &Program) -> CompiledFunction {
    Compiler::new().compile_program(program).expect("compile failed")
}

#[test]
fn test_compile_nil_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Nil)))],
    };
    let func = compile(&program);
    // Should have Pop (for stmt expr), Nil (implicit return), Return
    assert!(func.chunk.code.len() >= 3);
    assert!(matches!(func.chunk.code[0], OpCode::Nil));
}

#[test]
fn test_compile_int_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Int(42))))],
    };
    let func = compile(&program);
    assert!(matches!(func.chunk.code[0], OpCode::Constant(_)));
    // Check constant pool has Int(42)
    if let OpCode::Constant(idx) = &func.chunk.code[0] {
        if let Constant::Value(Value::Int(n)) = &func.chunk.constants[*idx as usize] {
            assert_eq!(*n, 42);
        } else {
            panic!("expected Int constant");
        }
    }
}

#[test]
fn test_compile_string_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Str("hello".to_string()))))],
    };
    let func = compile(&program);
    assert!(matches!(func.chunk.code[0], OpCode::Constant(_)));
}

#[test]
fn test_compile_bool_literals() {
    let program = Program {
        stmts: vec![
            s(Stmt::Expr(s(Expr::Bool(true)))),
            s(Stmt::Expr(s(Expr::Bool(false)))),
        ],
    };
    let func = compile(&program);
    assert!(matches!(func.chunk.code[0], OpCode::True));
    // After Pop, second bool
    assert!(matches!(func.chunk.code[2], OpCode::False));
}

#[test]
fn test_compile_binary_add() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(s(Expr::Int(1))),
            right: Box::new(s(Expr::Int(2))),
        })))],
    };
    let func = compile(&program);
    // Constant(1), Constant(2), Add, Pop, Nil, Return
    assert!(func.chunk.code.iter().any(|op| matches!(op, OpCode::Add)));
}

#[test]
fn test_compile_binary_ops() {
    for (op, expected) in [
        (BinaryOp::Sub, "SUB"),
        (BinaryOp::Mul, "MUL"),
        (BinaryOp::Div, "DIV"),
        (BinaryOp::Mod, "MOD"),
        (BinaryOp::Eq, "EQUAL"),
        (BinaryOp::Neq, "NOT_EQUAL"),
        (BinaryOp::Lt, "LESS"),
        (BinaryOp::Gt, "GREATER"),
        (BinaryOp::Le, "LESS_EQUAL"),
        (BinaryOp::Ge, "GREATER_EQUAL"),
    ] {
        let program = Program {
            stmts: vec![s(Stmt::Expr(s(Expr::Binary {
                op,
                left: Box::new(s(Expr::Int(1))),
                right: Box::new(s(Expr::Int(2))),
            })))],
        };
        let func = compile(&program);
        assert!(
            func.chunk.code.iter().any(|o| o.name() == expected),
            "expected {expected} opcode for {op}"
        );
    }
}

#[test]
fn test_compile_unary_neg() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Unary {
            op: UnaryOp::Neg,
            expr: Box::new(s(Expr::Int(5))),
        })))],
    };
    let func = compile(&program);
    assert!(func.chunk.code.iter().any(|op| matches!(op, OpCode::Negate)));
}

#[test]
fn test_compile_let_global() {
    let program = Program {
        stmts: vec![s(Stmt::Let {
            name: "x".to_string(),
            type_ann: None,
            value: s(Expr::Int(10)),
        })],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::DefineGlobal(_))));
}

#[test]
fn test_compile_variable_get() {
    let program = Program {
        stmts: vec![
            s(Stmt::Let {
                name: "x".to_string(),
                type_ann: None,
                value: s(Expr::Int(10)),
            }),
            s(Stmt::Expr(s(Expr::Ident("x".to_string())))),
        ],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::GetGlobal(_))));
}

#[test]
fn test_compile_if_else() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::If {
            condition: Box::new(s(Expr::Bool(true))),
            then_body: vec![s(Stmt::Expr(s(Expr::Int(1))))],
            else_body: Some(vec![s(Stmt::Expr(s(Expr::Int(2))))]),
        })))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::JumpIfFalse(_))));
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::Jump(_))));
}

#[test]
fn test_compile_while_loop() {
    let program = Program {
        stmts: vec![s(Stmt::While {
            condition: s(Expr::Bool(false)),
            body: vec![s(Stmt::Expr(s(Expr::Int(1))))],
        })],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::Loop(_))));
}

#[test]
fn test_compile_for_loop() {
    let program = Program {
        stmts: vec![s(Stmt::For {
            pattern: bl_core::ast::ForPattern::Single("i".to_string()),
            iter: s(Expr::List(vec![s(Expr::Int(1)), s(Expr::Int(2))])),
            body: vec![s(Stmt::Expr(s(Expr::Ident("i".to_string()))))],
            when_guard: None,
            else_body: None,
        })],
    };
    let func = compile(&program);
    assert!(func.chunk.code.iter().any(|op| matches!(op, OpCode::PushIter)));
    assert!(func.chunk.code.iter().any(|op| matches!(op, OpCode::IterNext(_))));
    assert!(func.chunk.code.iter().any(|op| matches!(op, OpCode::PopIter)));
}

#[test]
fn test_compile_list_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::List(vec![
            s(Expr::Int(1)),
            s(Expr::Int(2)),
            s(Expr::Int(3)),
        ]))))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::MakeList(3))));
}

#[test]
fn test_compile_record_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Record(vec![
            ("a".to_string(), s(Expr::Int(1))),
        ]))))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::MakeRecord(1))));
}

#[test]
fn test_compile_function_call() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Call {
            callee: Box::new(s(Expr::Ident("f".to_string()))),
            args: vec![Arg {
                name: None,
                value: s(Expr::Int(1)),
                spread: false,
            }],
        })))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::Call(1))));
}

#[test]
fn test_compile_fn_def() {
    let program = Program {
        stmts: vec![s(Stmt::Fn {
            name: "add".to_string(),
            params: vec![
                Param {
                    name: "a".to_string(),
                    type_ann: None,
                    default: None,
                    rest: false,
                },
                Param {
                    name: "b".to_string(),
                    type_ann: None,
                    default: None,
                    rest: false,
                },
            ],
            return_type: None,
            body: vec![s(Stmt::Return(Some(s(Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(s(Expr::Ident("a".to_string()))),
                right: Box::new(s(Expr::Ident("b".to_string()))),
            }))))],
            doc: None,
            is_generator: false,
            decorators: vec![],
            is_async: false,
            named_returns: vec![],
            where_clause: None,
        })],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::Closure(_))));
}

#[test]
fn test_compile_return() {
    let program = Program {
        stmts: vec![s(Stmt::Return(Some(s(Expr::Int(42)))))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::Return)));
}

#[test]
fn test_compile_pipe() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Pipe {
            left: Box::new(s(Expr::Int(1))),
            right: Box::new(s(Expr::Call {
                callee: Box::new(s(Expr::Ident("f".to_string()))),
                args: vec![Arg {
                    name: None,
                    value: s(Expr::Int(2)),
                    spread: false,
                }],
            })),
        })))],
    };
    let func = compile(&program);
    // Pipe desugars to call with 2 args
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::Call(2))));
}

#[test]
fn test_compile_range() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Range {
            start: Box::new(s(Expr::Int(1))),
            end: Box::new(s(Expr::Int(10))),
            inclusive: true,
        })))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::MakeRange(1))));
}

#[test]
fn test_compile_string_interp() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::StringInterp(vec![
            StringPart::Lit("hello ".to_string()),
            StringPart::Expr(s(Expr::Ident("name".to_string()))),
        ]))))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::StringInterp(2))));
}

#[test]
fn test_compile_short_circuit_and() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Binary {
            op: BinaryOp::And,
            left: Box::new(s(Expr::Bool(true))),
            right: Box::new(s(Expr::Bool(false))),
        })))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::JumpIfFalse(_))));
}

#[test]
fn test_compile_short_circuit_or() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Binary {
            op: BinaryOp::Or,
            left: Box::new(s(Expr::Bool(true))),
            right: Box::new(s(Expr::Bool(false))),
        })))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::JumpIfTrue(_))));
}

#[test]
fn test_compile_try_catch() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::TryCatch {
            body: vec![s(Stmt::Expr(s(Expr::Int(1))))],
            error_var: Some("e".to_string()),
            catch_body: vec![s(Stmt::Expr(s(Expr::Int(0))))],
        })))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::TryBegin(_))));
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::TryEnd)));
}

#[test]
fn test_compile_null_coalesce() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::NullCoalesce {
            left: Box::new(s(Expr::Nil)),
            right: Box::new(s(Expr::Int(42))),
        })))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::NullCoalesce(_))));
}

#[test]
fn test_compile_field_access() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Field {
            object: Box::new(s(Expr::Ident("obj".to_string()))),
            field: "x".to_string(),
            optional: false,
        })))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::GetField(_))));
}

#[test]
fn test_compile_optional_field() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Field {
            object: Box::new(s(Expr::Ident("obj".to_string()))),
            field: "x".to_string(),
            optional: true,
        })))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::GetFieldOpt(_))));
}

#[test]
fn test_compile_index_access() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Index {
            object: Box::new(s(Expr::Ident("arr".to_string()))),
            index: Box::new(s(Expr::Int(0))),
        })))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::GetIndex)));
}

#[test]
fn test_compile_set_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::SetLiteral(vec![
            s(Expr::Int(1)),
            s(Expr::Int(2)),
        ]))))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::MakeSet(2))));
}

#[test]
fn test_compile_assert() {
    let program = Program {
        stmts: vec![s(Stmt::Assert {
            condition: s(Expr::Bool(true)),
            message: None,
        })],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::AssertCheck)));
}

#[test]
fn test_compile_break_continue() {
    let program = Program {
        stmts: vec![s(Stmt::While {
            condition: s(Expr::Bool(true)),
            body: vec![s(Stmt::Break)],
        })],
    };
    let func = compile(&program);
    // Break compiles to a Jump that gets patched
    assert!(func.chunk.code.iter().any(|op| matches!(op, OpCode::Jump(_))));
}

#[test]
fn test_compile_lambda() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Lambda {
            params: vec![Param {
                name: "x".to_string(),
                type_ann: None,
                default: None,
                rest: false,
            }],
            body: Box::new(s(Expr::Ident("x".to_string()))),
        })))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::Closure(_))));
}

#[test]
fn test_disassemble() {
    let program = Program {
        stmts: vec![
            s(Stmt::Let {
                name: "x".to_string(),
                type_ann: None,
                value: s(Expr::Int(42)),
            }),
            s(Stmt::Expr(s(Expr::Ident("x".to_string())))),
        ],
    };
    let func = compile(&program);
    let output = disassemble_function(&func);
    assert!(output.contains("CONSTANT"));
    assert!(output.contains("DEFINE_GLOBAL"));
    assert!(output.contains("GET_GLOBAL"));
}

#[test]
fn test_compile_ternary() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Ternary {
            value: Box::new(s(Expr::Int(1))),
            condition: Box::new(s(Expr::Bool(true))),
            else_value: Box::new(s(Expr::Int(2))),
        })))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::JumpIfFalse(_))));
}

#[test]
fn test_compile_dna_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::DnaLit("ATCG".to_string()))))],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::MakeDna(_))));
}

#[test]
fn test_compile_destruct_let_list() {
    let program = Program {
        stmts: vec![s(Stmt::DestructLet {
            pattern: DestructPattern::List(vec!["a".to_string(), "b".to_string()]),
            value: s(Expr::List(vec![s(Expr::Int(1)), s(Expr::Int(2))])),
        })],
    };
    let func = compile(&program);
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::Dup)));
    assert!(func
        .chunk
        .code
        .iter()
        .any(|op| matches!(op, OpCode::GetIndex)));
}
