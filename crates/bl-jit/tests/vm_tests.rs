use bl_compiler::*;
use bl_core::ast::*;
use bl_core::error::Result;
use bl_core::span::{Span, Spanned};
use bl_core::value::{Arity, Value};
use bl_jit::*;

fn s<T>(node: T) -> Spanned<T> {
    Spanned::new(node, Span::new(0, 0))
}

/// Minimal builtin callback for testing.
struct TestCallback;

impl BuiltinCallback for TestCallback {
    fn call_builtin(&self, name: &str, args: Vec<Value>) -> Result<Value> {
        match name {
            "len" => match &args[0] {
                Value::List(l) => Ok(Value::Int(l.len() as i64)),
                Value::Str(s) => Ok(Value::Int(s.len() as i64)),
                _ => Ok(Value::Int(0)),
            },
            "type" => Ok(Value::Str(args[0].type_of().to_string())),
            "println" => Ok(Value::Nil),
            "print" => Ok(Value::Nil),
            "push" => {
                if let (Value::List(mut l), val) = (args[0].clone(), args[1].clone()) {
                    l.push(val);
                    Ok(Value::List(l))
                } else {
                    Ok(Value::Nil)
                }
            }
            _ => Ok(Value::Nil),
        }
    }

    fn builtin_list(&self) -> Vec<(String, Arity)> {
        vec![
            ("len".to_string(), Arity::Exact(1)),
            ("type".to_string(), Arity::Exact(1)),
            ("println".to_string(), Arity::AtLeast(0)),
            ("print".to_string(), Arity::AtLeast(0)),
            ("push".to_string(), Arity::Exact(2)),
        ]
    }
}

fn run_program(program: &Program) -> Result<Value> {
    let func = Compiler::new()
        .compile_program(program)
        .expect("compile failed");
    let registry = BuiltinRegistry::new(Box::new(TestCallback));
    let mut vm = Vm::new(registry);
    vm.execute(func)
}

#[test]
fn test_vm_nil() {
    let program = Program { stmts: vec![] };
    let result = run_program(&program).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_vm_int_constant() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Int(42))))],
    };
    // Program returns nil (last stmt is expr which gets popped, then implicit nil return)
    let result = run_program(&program).unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_vm_addition() {
    let program = Program {
        stmts: vec![s(Stmt::Let {
            name: "result".to_string(),
            type_ann: None,
            value: s(Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(s(Expr::Int(10))),
                right: Box::new(s(Expr::Int(32))),
            }),
        })],
    };
    let func = Compiler::new()
        .compile_program(&program)
        .expect("compile failed");
    let registry = BuiltinRegistry::new(Box::new(TestCallback));
    let mut vm = Vm::new(registry);
    vm.execute(func).unwrap();
    // No way to read globals directly, but it shouldn't error
}

#[test]
fn test_vm_arithmetic() {
    // Test all arithmetic ops by asserting they don't error
    for (op, left, right) in [
        (BinaryOp::Add, 10, 20),
        (BinaryOp::Sub, 30, 10),
        (BinaryOp::Mul, 5, 6),
        (BinaryOp::Div, 10, 2),
        (BinaryOp::Mod, 10, 3),
    ] {
        let program = Program {
            stmts: vec![s(Stmt::Expr(s(Expr::Binary {
                op,
                left: Box::new(s(Expr::Int(left))),
                right: Box::new(s(Expr::Int(right))),
            })))],
        };
        run_program(&program).expect(&format!("failed for op {:?}", op));
    }
}

#[test]
fn test_vm_comparison() {
    for (op, left, right) in [
        (BinaryOp::Eq, 1, 1),
        (BinaryOp::Neq, 1, 2),
        (BinaryOp::Lt, 1, 2),
        (BinaryOp::Gt, 2, 1),
        (BinaryOp::Le, 1, 1),
        (BinaryOp::Ge, 2, 1),
    ] {
        let program = Program {
            stmts: vec![s(Stmt::Expr(s(Expr::Binary {
                op,
                left: Box::new(s(Expr::Int(left))),
                right: Box::new(s(Expr::Int(right))),
            })))],
        };
        run_program(&program).expect(&format!("failed for op {:?}", op));
    }
}

#[test]
fn test_vm_negate() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Unary {
            op: UnaryOp::Neg,
            expr: Box::new(s(Expr::Int(42))),
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_not() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Unary {
            op: UnaryOp::Not,
            expr: Box::new(s(Expr::Bool(true))),
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_string_concat() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(s(Expr::Str("hello ".to_string()))),
            right: Box::new(s(Expr::Str("world".to_string()))),
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_if_true() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::If {
            condition: Box::new(s(Expr::Bool(true))),
            then_body: vec![s(Stmt::Expr(s(Expr::Int(1))))],
            else_body: Some(vec![s(Stmt::Expr(s(Expr::Int(2))))]),
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_if_false() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::If {
            condition: Box::new(s(Expr::Bool(false))),
            then_body: vec![s(Stmt::Expr(s(Expr::Int(1))))],
            else_body: Some(vec![s(Stmt::Expr(s(Expr::Int(2))))]),
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_while_loop() {
    // while false { ... } — doesn't execute body
    let program = Program {
        stmts: vec![s(Stmt::While {
            condition: s(Expr::Bool(false)),
            body: vec![s(Stmt::Expr(s(Expr::Int(1))))],
        })],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_for_loop() {
    let program = Program {
        stmts: vec![s(Stmt::For {
            pattern: bl_core::ast::ForPattern::Single("i".to_string()),
            iter: s(Expr::List(vec![s(Expr::Int(1)), s(Expr::Int(2)), s(Expr::Int(3))])),
            body: vec![s(Stmt::Expr(s(Expr::Ident("i".to_string()))))],
            when_guard: None,
            else_body: None,
        })],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_for_range() {
    let program = Program {
        stmts: vec![s(Stmt::For {
            pattern: bl_core::ast::ForPattern::Single("i".to_string()),
            iter: s(Expr::Range {
                start: Box::new(s(Expr::Int(0))),
                end: Box::new(s(Expr::Int(5))),
                inclusive: false,
            }),
            body: vec![s(Stmt::Expr(s(Expr::Ident("i".to_string()))))],
            when_guard: None,
            else_body: None,
        })],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_list_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::List(vec![
            s(Expr::Int(1)),
            s(Expr::Int(2)),
            s(Expr::Int(3)),
        ]))))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_record_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Record(vec![
            ("x".to_string(), s(Expr::Int(1))),
            ("y".to_string(), s(Expr::Int(2))),
        ]))))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_globals() {
    let program = Program {
        stmts: vec![
            s(Stmt::Let {
                name: "x".to_string(),
                type_ann: None,
                value: s(Expr::Int(42)),
            }),
            s(Stmt::Assign {
                name: "x".to_string(),
                value: s(Expr::Int(100)),
            }),
        ],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_function_def_and_call() {
    let program = Program {
        stmts: vec![
            s(Stmt::Fn {
                name: "double".to_string(),
                params: vec![Param {
                    name: "n".to_string(),
                    type_ann: None,
                    default: None,
                    rest: false,
                }],
                return_type: None,
                body: vec![s(Stmt::Return(Some(s(Expr::Binary {
                    op: BinaryOp::Mul,
                    left: Box::new(s(Expr::Ident("n".to_string()))),
                    right: Box::new(s(Expr::Int(2))),
                }))))],
                doc: None,
                is_generator: false,
                decorators: vec![],
                is_async: false,
                named_returns: vec![],
                where_clause: None,
            }),
            s(Stmt::Expr(s(Expr::Call {
                callee: Box::new(s(Expr::Ident("double".to_string()))),
                args: vec![Arg {
                    name: None,
                    value: s(Expr::Int(21)),
                    spread: false,
                }],
            }))),
        ],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_range() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Range {
            start: Box::new(s(Expr::Int(1))),
            end: Box::new(s(Expr::Int(10))),
            inclusive: true,
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_short_circuit_and() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Binary {
            op: BinaryOp::And,
            left: Box::new(s(Expr::Bool(false))),
            right: Box::new(s(Expr::Int(42))), // should not be evaluated
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_short_circuit_or() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Binary {
            op: BinaryOp::Or,
            left: Box::new(s(Expr::Bool(true))),
            right: Box::new(s(Expr::Int(42))), // should not be evaluated
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_null_coalesce() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::NullCoalesce {
            left: Box::new(s(Expr::Nil)),
            right: Box::new(s(Expr::Int(42))),
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_string_interp() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::StringInterp(vec![
            StringPart::Lit("hello ".to_string()),
            StringPart::Expr(s(Expr::Int(42))),
        ]))))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_set_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::SetLiteral(vec![
            s(Expr::Int(1)),
            s(Expr::Int(2)),
            s(Expr::Int(1)), // duplicate
        ]))))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_try_catch() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::TryCatch {
            body: vec![s(Stmt::Expr(s(Expr::Int(1))))],
            error_var: Some("e".to_string()),
            catch_body: vec![s(Stmt::Expr(s(Expr::Int(0))))],
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_assert_pass() {
    let program = Program {
        stmts: vec![s(Stmt::Assert {
            condition: s(Expr::Bool(true)),
            message: None,
        })],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_assert_fail() {
    let program = Program {
        stmts: vec![s(Stmt::Assert {
            condition: s(Expr::Bool(false)),
            message: Some(s(Expr::Str("test failed".to_string()))),
        })],
    };
    let result = run_program(&program);
    assert!(result.is_err());
}

#[test]
fn test_vm_division_by_zero() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Binary {
            op: BinaryOp::Div,
            left: Box::new(s(Expr::Int(1))),
            right: Box::new(s(Expr::Int(0))),
        })))],
    };
    let result = run_program(&program);
    assert!(result.is_err());
}

#[test]
fn test_vm_dna_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::DnaLit("ATCG".to_string()))))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_rna_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::RnaLit("AUCG".to_string()))))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_protein_literal() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::ProteinLit("MVILK".to_string()))))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_field_access() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Field {
            object: Box::new(s(Expr::Record(vec![("x".to_string(), s(Expr::Int(42)))]))),
            field: "x".to_string(),
            optional: false,
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_index_access() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Index {
            object: Box::new(s(Expr::List(vec![
                s(Expr::Int(10)),
                s(Expr::Int(20)),
                s(Expr::Int(30)),
            ]))),
            index: Box::new(s(Expr::Int(1))),
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_builtin_call() {
    // Call len() which is registered as a test builtin
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Call {
            callee: Box::new(s(Expr::Ident("len".to_string()))),
            args: vec![Arg {
                name: None,
                value: s(Expr::List(vec![s(Expr::Int(1)), s(Expr::Int(2))])),
                spread: false,
            }],
        })))],
    };
    let func = Compiler::new()
        .compile_program(&program)
        .expect("compile failed");
    let registry = BuiltinRegistry::new(Box::new(TestCallback));
    let mut vm = Vm::new(registry);
    // Register builtins as globals
    vm.define_global(
        "len".to_string(),
        Value::NativeFunction {
            name: "len".to_string(),
            arity: Arity::Exact(1),
        },
    );
    vm.execute(func).unwrap();
}

#[test]
fn test_vm_pipe_desugar() {
    // 5 |> double → double(5)
    let program = Program {
        stmts: vec![
            s(Stmt::Fn {
                name: "double".to_string(),
                params: vec![Param {
                    name: "n".to_string(),
                    type_ann: None,
                    default: None,
                    rest: false,
                }],
                return_type: None,
                body: vec![s(Stmt::Return(Some(s(Expr::Binary {
                    op: BinaryOp::Mul,
                    left: Box::new(s(Expr::Ident("n".to_string()))),
                    right: Box::new(s(Expr::Int(2))),
                }))))],
                doc: None,
                is_generator: false,
                decorators: vec![],
                is_async: false,
                named_returns: vec![],
                where_clause: None,
            }),
            s(Stmt::Expr(s(Expr::Pipe {
                left: Box::new(s(Expr::Int(5))),
                right: Box::new(s(Expr::Ident("double".to_string()))),
            }))),
        ],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_ternary() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Ternary {
            value: Box::new(s(Expr::Int(1))),
            condition: Box::new(s(Expr::Bool(true))),
            else_value: Box::new(s(Expr::Int(2))),
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_enum_unit() {
    let program = Program {
        stmts: vec![s(Stmt::Enum {
            name: "Color".to_string(),
            variants: vec![
                EnumVariant {
                    name: "Red".to_string(),
                    fields: vec![],
                },
                EnumVariant {
                    name: "Blue".to_string(),
                    fields: vec![],
                },
            ],
        })],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_destruct_let() {
    let program = Program {
        stmts: vec![s(Stmt::DestructLet {
            pattern: DestructPattern::List(vec!["a".to_string(), "b".to_string()]),
            value: s(Expr::List(vec![s(Expr::Int(1)), s(Expr::Int(2))])),
        })],
    };
    run_program(&program).unwrap();
}

// ── JIT Performance / Hot Loop Tests ──────────────────────────────

#[test]
fn test_vm_loop_sum() {
    // Sum 1..100 in a while loop — tests hot loop execution
    let program = Program {
        stmts: vec![
            s(Stmt::Let {
                name: "sum".to_string(),
                type_ann: None,
                value: s(Expr::Int(0)),
            }),
            s(Stmt::Let {
                name: "i".to_string(),
                type_ann: None,
                value: s(Expr::Int(1)),
            }),
            s(Stmt::While {
                condition: s(Expr::Binary {
                    op: BinaryOp::Le,
                    left: Box::new(s(Expr::Ident("i".to_string()))),
                    right: Box::new(s(Expr::Int(100))),
                }),
                body: vec![
                    s(Stmt::Assign {
                        name: "sum".to_string(),
                        value: s(Expr::Binary {
                            op: BinaryOp::Add,
                            left: Box::new(s(Expr::Ident("sum".to_string()))),
                            right: Box::new(s(Expr::Ident("i".to_string()))),
                        }),
                    }),
                    s(Stmt::Assign {
                        name: "i".to_string(),
                        value: s(Expr::Binary {
                            op: BinaryOp::Add,
                            left: Box::new(s(Expr::Ident("i".to_string()))),
                            right: Box::new(s(Expr::Int(1))),
                        }),
                    }),
                ],
            }),
        ],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_nested_loop() {
    // Nested for loops
    let program = Program {
        stmts: vec![
            s(Stmt::Let {
                name: "count".to_string(),
                type_ann: None,
                value: s(Expr::Int(0)),
            }),
            s(Stmt::For {
                pattern: ForPattern::Single("i".to_string()),
                iter: s(Expr::Range {
                    start: Box::new(s(Expr::Int(0))),
                    end: Box::new(s(Expr::Int(10))),
                    inclusive: false,
                }),
                body: vec![s(Stmt::For {
                    pattern: ForPattern::Single("j".to_string()),
                    iter: s(Expr::Range {
                        start: Box::new(s(Expr::Int(0))),
                        end: Box::new(s(Expr::Int(10))),
                        inclusive: false,
                    }),
                    body: vec![s(Stmt::Assign {
                        name: "count".to_string(),
                        value: s(Expr::Binary {
                            op: BinaryOp::Add,
                            left: Box::new(s(Expr::Ident("count".to_string()))),
                            right: Box::new(s(Expr::Int(1))),
                        }),
                    })],
                    when_guard: None,
                    else_body: None,
                })],
                when_guard: None,
                else_body: None,
            }),
        ],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_recursive_function() {
    let program = Program {
        stmts: vec![
            s(Stmt::Fn {
                name: "fib".to_string(),
                params: vec![Param {
                    name: "n".to_string(),
                    type_ann: None,
                    default: None,
                    rest: false,
                }],
                return_type: None,
                body: vec![s(Stmt::Expr(s(Expr::If {
                    condition: Box::new(s(Expr::Binary {
                        op: BinaryOp::Le,
                        left: Box::new(s(Expr::Ident("n".to_string()))),
                        right: Box::new(s(Expr::Int(1))),
                    })),
                    then_body: vec![s(Stmt::Return(Some(s(Expr::Ident("n".to_string())))))],
                    else_body: Some(vec![s(Stmt::Return(Some(s(Expr::Binary {
                        op: BinaryOp::Add,
                        left: Box::new(s(Expr::Call {
                            callee: Box::new(s(Expr::Ident("fib".to_string()))),
                            args: vec![Arg {
                                name: None,
                                value: s(Expr::Binary {
                                    op: BinaryOp::Sub,
                                    left: Box::new(s(Expr::Ident("n".to_string()))),
                                    right: Box::new(s(Expr::Int(1))),
                                }),
                                spread: false,
                            }],
                        })),
                        right: Box::new(s(Expr::Call {
                            callee: Box::new(s(Expr::Ident("fib".to_string()))),
                            args: vec![Arg {
                                name: None,
                                value: s(Expr::Binary {
                                    op: BinaryOp::Sub,
                                    left: Box::new(s(Expr::Ident("n".to_string()))),
                                    right: Box::new(s(Expr::Int(2))),
                                }),
                                spread: false,
                            }],
                        })),
                    }))))]),
                })))],
                doc: None,
                is_generator: false,
                decorators: vec![],
                is_async: false,
                named_returns: vec![],
                where_clause: None,
            }),
            s(Stmt::Expr(s(Expr::Call {
                callee: Box::new(s(Expr::Ident("fib".to_string()))),
                args: vec![Arg {
                    name: None,
                    value: s(Expr::Int(10)),
                    spread: false,
                }],
            }))),
        ],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_multiple_functions() {
    let program = Program {
        stmts: vec![
            s(Stmt::Fn {
                name: "add".to_string(),
                params: vec![
                    Param { name: "a".to_string(), type_ann: None, default: None, rest: false },
                    Param { name: "b".to_string(), type_ann: None, default: None, rest: false },
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
            }),
            s(Stmt::Fn {
                name: "mul".to_string(),
                params: vec![
                    Param { name: "a".to_string(), type_ann: None, default: None, rest: false },
                    Param { name: "b".to_string(), type_ann: None, default: None, rest: false },
                ],
                return_type: None,
                body: vec![s(Stmt::Return(Some(s(Expr::Binary {
                    op: BinaryOp::Mul,
                    left: Box::new(s(Expr::Ident("a".to_string()))),
                    right: Box::new(s(Expr::Ident("b".to_string()))),
                }))))],
                doc: None,
                is_generator: false,
                decorators: vec![],
                is_async: false,
                named_returns: vec![],
                where_clause: None,
            }),
            // add(mul(3, 4), 5) = 17
            s(Stmt::Expr(s(Expr::Call {
                callee: Box::new(s(Expr::Ident("add".to_string()))),
                args: vec![
                    Arg {
                        name: None,
                        value: s(Expr::Call {
                            callee: Box::new(s(Expr::Ident("mul".to_string()))),
                            args: vec![
                                Arg { name: None, value: s(Expr::Int(3)), spread: false },
                                Arg { name: None, value: s(Expr::Int(4)), spread: false },
                            ],
                        }),
                        spread: false,
                    },
                    Arg { name: None, value: s(Expr::Int(5)), spread: false },
                ],
            }))),
        ],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_try_catch_error() {
    // Division by zero caught by try-catch
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::TryCatch {
            body: vec![s(Stmt::Expr(s(Expr::Binary {
                op: BinaryOp::Div,
                left: Box::new(s(Expr::Int(1))),
                right: Box::new(s(Expr::Int(0))),
            })))],
            error_var: Some("e".to_string()),
            catch_body: vec![s(Stmt::Expr(s(Expr::Int(999))))],
        })))],
    };
    // Should not error — catch handles it
    run_program(&program).unwrap();
}

#[test]
fn test_vm_list_operations() {
    // Build and index a list
    let program = Program {
        stmts: vec![
            s(Stmt::Let {
                name: "xs".to_string(),
                type_ann: None,
                value: s(Expr::List(vec![
                    s(Expr::Int(10)),
                    s(Expr::Int(20)),
                    s(Expr::Int(30)),
                ])),
            }),
            // xs[0] + xs[2]
            s(Stmt::Expr(s(Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(s(Expr::Index {
                    object: Box::new(s(Expr::Ident("xs".to_string()))),
                    index: Box::new(s(Expr::Int(0))),
                })),
                right: Box::new(s(Expr::Index {
                    object: Box::new(s(Expr::Ident("xs".to_string()))),
                    index: Box::new(s(Expr::Int(2))),
                })),
            }))),
        ],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_conditional_chain() {
    // if false { 1 } else { if true { 2 } else { 3 } }
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::If {
            condition: Box::new(s(Expr::Bool(false))),
            then_body: vec![s(Stmt::Expr(s(Expr::Int(1))))],
            else_body: Some(vec![s(Stmt::Expr(s(Expr::If {
                condition: Box::new(s(Expr::Bool(true))),
                then_body: vec![s(Stmt::Expr(s(Expr::Int(2))))],
                else_body: Some(vec![s(Stmt::Expr(s(Expr::Int(3))))]),
            })))]),
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_float_arithmetic() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(s(Expr::Float(1.5))),
            right: Box::new(s(Expr::Float(2.5))),
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_mixed_int_float() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Binary {
            op: BinaryOp::Mul,
            left: Box::new(s(Expr::Int(3))),
            right: Box::new(s(Expr::Float(1.5))),
        })))],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_upvalue_capture() {
    // Closure that captures a variable from outer scope
    let program = Program {
        stmts: vec![
            s(Stmt::Let {
                name: "x".to_string(),
                type_ann: None,
                value: s(Expr::Int(100)),
            }),
            s(Stmt::Fn {
                name: "get_x".to_string(),
                params: vec![],
                return_type: None,
                body: vec![s(Stmt::Return(Some(s(Expr::Ident("x".to_string())))))],
                doc: None,
                is_generator: false,
                decorators: vec![],
                is_async: false,
                named_returns: vec![],
                where_clause: None,
            }),
            s(Stmt::Expr(s(Expr::Call {
                callee: Box::new(s(Expr::Ident("get_x".to_string()))),
                args: vec![],
            }))),
        ],
    };
    run_program(&program).unwrap();
}

#[test]
fn test_vm_formula() {
    let program = Program {
        stmts: vec![s(Stmt::Expr(s(Expr::Formula(Box::new(s(Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(s(Expr::Ident("x".to_string()))),
            right: Box::new(s(Expr::Int(1))),
        }))))))],
    };
    run_program(&program).unwrap();
}
