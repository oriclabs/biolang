use bl_core::ast::*;
use bl_lexer::Lexer;
use bl_parser::Parser;

fn parse(input: &str) -> Program {
    let tokens = Lexer::new(input).tokenize().unwrap();
    let result = Parser::new(tokens).parse().unwrap();
    assert!(
        !result.has_errors(),
        "unexpected parse errors: {:?}",
        result.errors
    );
    result.program
}

fn parse_has_errors(input: &str) -> bool {
    let tok = Lexer::new(input).tokenize();
    match tok {
        Err(_) => true,
        Ok(tokens) => {
            let result = Parser::new(tokens).parse().unwrap();
            result.has_errors()
        }
    }
}

// ============================================================
// Existing tests (ported from inline mod tests)
// ============================================================

#[test]
fn test_let_simple() {
    let prog = parse("let x = 42");
    assert_eq!(prog.stmts.len(), 1);
    match &prog.stmts[0].node {
        Stmt::Let { name, value, .. } => {
            assert_eq!(name, "x");
            assert_eq!(value.node, Expr::Int(42));
        }
        other => panic!("expected Let, got {other:?}"),
    }
}

#[test]
fn test_binary_precedence() {
    let prog = parse("1 + 2 * 3");
    assert_eq!(prog.stmts.len(), 1);
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Binary { op, left, right } => {
                assert_eq!(*op, BinaryOp::Add);
                assert_eq!(left.node, Expr::Int(1));
                match &right.node {
                    Expr::Binary { op, left, right } => {
                        assert_eq!(*op, BinaryOp::Mul);
                        assert_eq!(left.node, Expr::Int(2));
                        assert_eq!(right.node, Expr::Int(3));
                    }
                    other => panic!("expected Binary, got {other:?}"),
                }
            }
            other => panic!("expected Binary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_pipe() {
    let prog = parse("x |> f()");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Pipe { left, right } => {
                assert!(matches!(left.node, Expr::Ident(_)));
                assert!(matches!(right.node, Expr::Call { .. }));
            }
            other => panic!("expected Pipe, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_lambda() {
    let prog = parse("|n| n * 2");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Lambda { params, body } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "n");
                assert!(matches!(body.node, Expr::Binary { .. }));
            }
            other => panic!("expected Lambda, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_milestone_pipe_lambda() {
    let prog = parse("let x = 10 |> |n| n * 2");
    match &prog.stmts[0].node {
        Stmt::Let { name, value, .. } => {
            assert_eq!(name, "x");
            match &value.node {
                Expr::Pipe { left, right } => {
                    assert_eq!(left.node, Expr::Int(10));
                    assert!(matches!(right.node, Expr::Lambda { .. }));
                }
                other => panic!("expected Pipe, got {other:?}"),
            }
        }
        other => panic!("expected Let, got {other:?}"),
    }
}

#[test]
fn test_function_def() {
    let prog = parse("fn add(a, b) { a + b }");
    match &prog.stmts[0].node {
        Stmt::Fn {
            name, params, body, ..
        } => {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            assert_eq!(body.len(), 1);
        }
        other => panic!("expected Fn, got {other:?}"),
    }
}

#[test]
fn test_named_args() {
    let prog = parse("f(a, key: value)");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 2);
                assert!(args[0].name.is_none());
                assert_eq!(args[1].name.as_deref(), Some("key"));
            }
            other => panic!("expected Call, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_if_expr() {
    let prog = parse("if x > 0 { x } else { -x }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => {
            assert!(matches!(expr.node, Expr::If { .. }));
        }
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_list_literal() {
    let prog = parse("[1, 2, 3]");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::List(items) => assert_eq!(items.len(), 3),
            other => panic!("expected List, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_multiline_pipe() {
    let prog = parse("x |>\n  f() |>\n  g()");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Pipe { right, .. } => {
                assert!(matches!(right.node, Expr::Call { .. }));
            }
            other => panic!("expected Pipe, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_field_access() {
    let prog = parse("obj.field");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Field { field, .. } => assert_eq!(field, "field"),
            other => panic!("expected Field, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_while_loop() {
    let prog = parse("while x > 0 { x = x - 1 }");
    match &prog.stmts[0].node {
        Stmt::While { condition, body } => {
            assert!(matches!(condition.node, Expr::Binary { .. }));
            assert_eq!(body.len(), 1);
        }
        other => panic!("expected While, got {other:?}"),
    }
}

#[test]
fn test_break_continue() {
    let prog = parse("for i in [1,2,3] { break }");
    match &prog.stmts[0].node {
        Stmt::For { body, .. } => {
            assert!(matches!(body[0].node, Stmt::Break));
        }
        other => panic!("expected For, got {other:?}"),
    }

    let prog = parse("for i in [1,2,3] { continue }");
    match &prog.stmts[0].node {
        Stmt::For { body, .. } => {
            assert!(matches!(body[0].node, Stmt::Continue));
        }
        other => panic!("expected For, got {other:?}"),
    }
}

#[test]
fn test_compound_assign() {
    let prog = parse("x += 1");
    match &prog.stmts[0].node {
        Stmt::Assign { name, value } => {
            assert_eq!(name, "x");
            match &value.node {
                Expr::Binary { op, .. } => assert_eq!(*op, BinaryOp::Add),
                other => panic!("expected Binary, got {other:?}"),
            }
        }
        other => panic!("expected Assign, got {other:?}"),
    }
}

#[test]
fn test_null_coalesce() {
    let prog = parse("x ?? 42");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => {
            assert!(matches!(expr.node, Expr::NullCoalesce { .. }));
        }
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_try_catch() {
    let prog = parse("try { risky() } catch e { e }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::TryCatch {
                body,
                error_var,
                catch_body,
            } => {
                assert_eq!(body.len(), 1);
                assert_eq!(error_var.as_deref(), Some("e"));
                assert_eq!(catch_body.len(), 1);
            }
            other => panic!("expected TryCatch, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_fstring() {
    let prog = parse(r#"f"hello {name}""#);
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::StringInterp(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], StringPart::Lit(s) if s == "hello "));
                assert!(matches!(&parts[1], StringPart::Expr(_)));
            }
            other => panic!("expected StringInterp, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_destruct_list() {
    let prog = parse("let [a, b, c] = [1, 2, 3]");
    match &prog.stmts[0].node {
        Stmt::DestructLet { pattern, .. } => match pattern {
            DestructPattern::List(names) => {
                assert_eq!(names, &["a", "b", "c"]);
            }
            other => panic!("expected List pattern, got {other:?}"),
        },
        other => panic!("expected DestructLet, got {other:?}"),
    }
}

#[test]
fn test_destruct_record() {
    let prog = parse("let {x, y} = rec");
    match &prog.stmts[0].node {
        Stmt::DestructLet { pattern, .. } => match pattern {
            DestructPattern::Record(names) => {
                assert_eq!(names, &["x", "y"]);
            }
            other => panic!("expected Record pattern, got {other:?}"),
        },
        other => panic!("expected DestructLet, got {other:?}"),
    }
}

// ============================================================
// New comprehensive tests
// ============================================================

// ── for loop ───────────────────────────────────────────────────

#[test]
fn test_for_loop_simple() {
    let prog = parse("for x in items { x }");
    match &prog.stmts[0].node {
        Stmt::For { pattern, iter, body, .. } => {
            assert!(matches!(pattern, ForPattern::Single(n) if n == "x"));
            assert!(matches!(iter.node, Expr::Ident(ref n) if n == "items"));
            assert_eq!(body.len(), 1);
        }
        other => panic!("expected For, got {other:?}"),
    }
}

#[test]
fn test_for_loop_list_destructure() {
    let prog = parse("for [a, b] in pairs { a }");
    match &prog.stmts[0].node {
        Stmt::For { pattern, .. } => match pattern {
            ForPattern::ListDestr(names) => {
                assert_eq!(names, &["a", "b"]);
            }
            other => panic!("expected ListDestr, got {other:?}"),
        },
        other => panic!("expected For, got {other:?}"),
    }
}

#[test]
fn test_for_loop_record_destructure() {
    let prog = parse("for {x, y} in points { x }");
    match &prog.stmts[0].node {
        Stmt::For { pattern, .. } => match pattern {
            ForPattern::RecordDestr(names) => {
                assert_eq!(names, &["x", "y"]);
            }
            other => panic!("expected RecordDestr, got {other:?}"),
        },
        other => panic!("expected For, got {other:?}"),
    }
}

// ── match expression ──────────────────────────────────────────

#[test]
fn test_match_expr() {
    let prog = parse("match x { 1 => \"one\", _ => \"other\" }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Match { expr, arms } => {
                assert!(matches!(expr.node, Expr::Ident(ref n) if n == "x"));
                assert_eq!(arms.len(), 2);
                assert!(matches!(arms[0].pattern.node, Pattern::Literal(_)));
                assert!(matches!(arms[1].pattern.node, Pattern::Wildcard));
            }
            other => panic!("expected Match, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_match_with_guard() {
    let prog = parse("match val { n if n > 0 => n, _ => 0 }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Match { arms, .. } => {
                assert!(arms[0].guard.is_some());
                assert!(arms[1].guard.is_none());
            }
            other => panic!("expected Match, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_match_enum_variant_pattern() {
    let prog = parse("match x { Some(v) => v, None => 0 }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Match { arms, .. } => {
                match &arms[0].pattern.node {
                    Pattern::EnumVariant { variant, bindings } => {
                        assert_eq!(variant, "Some");
                        assert_eq!(bindings, &["v"]);
                    }
                    other => panic!("expected EnumVariant, got {other:?}"),
                }
                assert!(matches!(arms[1].pattern.node, Pattern::Ident(ref n) if n == "None"));
            }
            other => panic!("expected Match, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_match_type_pattern() {
    let prog = parse("match x { Int(n) => n, Str(s) => 0 }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Match { arms, .. } => {
                match &arms[0].pattern.node {
                    Pattern::TypePattern { type_name, binding } => {
                        assert_eq!(type_name, "Int");
                        assert_eq!(binding.as_deref(), Some("n"));
                    }
                    other => panic!("expected TypePattern, got {other:?}"),
                }
            }
            other => panic!("expected Match, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── return statement ──────────────────────────────────────────

#[test]
fn test_return_with_value() {
    let prog = parse("fn foo() { return 42 }");
    match &prog.stmts[0].node {
        Stmt::Fn { body, .. } => match &body[0].node {
            Stmt::Return(Some(expr)) => {
                assert_eq!(expr.node, Expr::Int(42));
            }
            other => panic!("expected Return(Some), got {other:?}"),
        },
        other => panic!("expected Fn, got {other:?}"),
    }
}

#[test]
fn test_return_without_value() {
    let prog = parse("fn foo() {\nreturn\n}");
    match &prog.stmts[0].node {
        Stmt::Fn { body, .. } => {
            assert!(matches!(body[0].node, Stmt::Return(None)));
        }
        other => panic!("expected Fn, got {other:?}"),
    }
}

// ── import ────────────────────────────────────────────────────

#[test]
fn test_import_simple() {
    let prog = parse(r#"import "path/to/module""#);
    match &prog.stmts[0].node {
        Stmt::Import { path, alias } => {
            assert_eq!(path, "path/to/module");
            assert!(alias.is_none());
        }
        other => panic!("expected Import, got {other:?}"),
    }
}

#[test]
fn test_import_with_alias() {
    let prog = parse(r#"import "utils" as u"#);
    match &prog.stmts[0].node {
        Stmt::Import { path, alias } => {
            assert_eq!(path, "utils");
            assert_eq!(alias.as_deref(), Some("u"));
        }
        other => panic!("expected Import, got {other:?}"),
    }
}

// ── struct definition ─────────────────────────────────────────

#[test]
fn test_struct_definition() {
    let prog = parse("struct Point { x: Int, y: Int }");
    match &prog.stmts[0].node {
        Stmt::Struct { name, fields } => {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_eq!(fields[0].type_ann.as_ref().unwrap().name, "Int");
            assert_eq!(fields[1].name, "y");
        }
        other => panic!("expected Struct, got {other:?}"),
    }
}

#[test]
fn test_struct_with_default() {
    let prog = parse("struct Config { debug = false }");
    match &prog.stmts[0].node {
        Stmt::Struct { name, fields } => {
            assert_eq!(name, "Config");
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "debug");
            assert!(fields[0].default.is_some());
        }
        other => panic!("expected Struct, got {other:?}"),
    }
}

// ── impl block ────────────────────────────────────────────────

#[test]
fn test_impl_block() {
    let prog = parse("impl Point {\n  fn x(self) { self.x }\n}");
    match &prog.stmts[0].node {
        Stmt::Impl {
            type_name,
            trait_name,
            methods,
        } => {
            assert_eq!(type_name, "Point");
            assert!(trait_name.is_none());
            assert_eq!(methods.len(), 1);
            match &methods[0].node {
                Stmt::Fn { name, params, .. } => {
                    assert_eq!(name, "x");
                    assert_eq!(params[0].name, "self");
                }
                other => panic!("expected Fn, got {other:?}"),
            }
        }
        other => panic!("expected Impl, got {other:?}"),
    }
}

/// `impl Trait for Type` is not currently supported because `for` is lexed as
/// `TokenKind::For` (keyword) but the parser checks for `TokenKind::Ident("for")`.
/// This causes an infinite loop in error recovery, so this test is intentionally
/// skipped. The parser needs to be updated to handle `for` as a contextual keyword
/// in this position.
#[test]
#[ignore = "causes infinite loop in parser error recovery — for is a keyword, not an ident"]
fn test_impl_trait_for_type_limitation() {
    // Would need parser fix to handle TokenKind::For in impl context
    let _prog = parse("impl Display for Point {\n  fn show(self) { self.x }\n}");
}

// ── nested if / else if / else chains ─────────────────────────

#[test]
fn test_if_else_if_else() {
    let prog = parse("if x > 10 { 1 } else if x > 5 { 2 } else { 3 }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::If {
                then_body,
                else_body,
                ..
            } => {
                assert_eq!(then_body.len(), 1);
                let else_stmts = else_body.as_ref().unwrap();
                assert_eq!(else_stmts.len(), 1);
                // The else-if is wrapped as a Stmt::Expr(If {...})
                match &else_stmts[0].node {
                    Stmt::Expr(inner) => {
                        assert!(matches!(inner.node, Expr::If { .. }));
                    }
                    other => panic!("expected nested If, got {other:?}"),
                }
            }
            other => panic!("expected If, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_if_no_else() {
    let prog = parse("if true { 1 }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::If { else_body, .. } => {
                assert!(else_body.is_none());
            }
            other => panic!("expected If, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── nested function calls ─────────────────────────────────────

#[test]
fn test_nested_function_calls() {
    let prog = parse("f(g(x))");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Call { callee, args } => {
                assert!(matches!(callee.node, Expr::Ident(ref n) if n == "f"));
                assert_eq!(args.len(), 1);
                match &args[0].value.node {
                    Expr::Call { callee, args } => {
                        assert!(matches!(callee.node, Expr::Ident(ref n) if n == "g"));
                        assert_eq!(args.len(), 1);
                        assert!(matches!(args[0].value.node, Expr::Ident(ref n) if n == "x"));
                    }
                    other => panic!("expected inner Call, got {other:?}"),
                }
            }
            other => panic!("expected Call, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── pipe into lambda ──────────────────────────────────────────

#[test]
fn test_pipe_into_lambda() {
    let prog = parse("x |> |n| n + 1");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Pipe { left, right } => {
                assert!(matches!(left.node, Expr::Ident(ref n) if n == "x"));
                assert!(matches!(right.node, Expr::Lambda { .. }));
            }
            other => panic!("expected Pipe, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── pipe chain ────────────────────────────────────────────────

#[test]
fn test_pipe_chain() {
    let prog = parse("a |> b() |> c() |> d()");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => {
            // Pipe is left-associative, so the outermost Pipe has right = d()
            match &expr.node {
                Expr::Pipe { left, right } => {
                    assert!(matches!(right.node, Expr::Call { .. }));
                    // left should be another Pipe
                    match &left.node {
                        Expr::Pipe { left, right } => {
                            assert!(matches!(right.node, Expr::Call { .. }));
                            // innermost: a |> b()
                            match &left.node {
                                Expr::Pipe { left, right } => {
                                    assert!(matches!(left.node, Expr::Ident(ref n) if n == "a"));
                                    assert!(matches!(right.node, Expr::Call { .. }));
                                }
                                other => panic!("expected innermost Pipe, got {other:?}"),
                            }
                        }
                        other => panic!("expected middle Pipe, got {other:?}"),
                    }
                }
                other => panic!("expected Pipe, got {other:?}"),
            }
        }
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── unary operators ───────────────────────────────────────────

#[test]
fn test_unary_minus() {
    let prog = parse("-x");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Unary { op, expr } => {
                assert_eq!(*op, UnaryOp::Neg);
                assert!(matches!(expr.node, Expr::Ident(ref n) if n == "x"));
            }
            other => panic!("expected Unary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_unary_not() {
    let prog = parse("!x");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Unary { op, expr } => {
                assert_eq!(*op, UnaryOp::Not);
                assert!(matches!(expr.node, Expr::Ident(ref n) if n == "x"));
            }
            other => panic!("expected Unary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_unary_minus_literal() {
    let prog = parse("-42");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Unary { op, expr } => {
                assert_eq!(*op, UnaryOp::Neg);
                assert_eq!(expr.node, Expr::Int(42));
            }
            other => panic!("expected Unary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── string interpolation with expression ──────────────────────

#[test]
fn test_fstring_with_expression() {
    let prog = parse(r#"f"hello {name + 1}""#);
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::StringInterp(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], StringPart::Lit(s) if s == "hello "));
                match &parts[1] {
                    StringPart::Expr(e) => {
                        assert!(matches!(e.node, Expr::Binary { .. }));
                    }
                    other => panic!("expected Expr part, got {other:?}"),
                }
            }
            other => panic!("expected StringInterp, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_fstring_multiple_parts() {
    let prog = parse(r#"f"{a} and {b}""#);
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::StringInterp(parts) => {
                // {a}, " and ", {b}
                assert_eq!(parts.len(), 3);
                assert!(matches!(&parts[0], StringPart::Expr(_)));
                assert!(matches!(&parts[1], StringPart::Lit(s) if s == " and "));
                assert!(matches!(&parts[2], StringPart::Expr(_)));
            }
            other => panic!("expected StringInterp, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── record literal ────────────────────────────────────────────

#[test]
fn test_record_literal() {
    let prog = parse("{x: 1, y: 2}");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Record(fields) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "x");
                assert_eq!(fields[0].1.node, Expr::Int(1));
                assert_eq!(fields[1].0, "y");
                assert_eq!(fields[1].1.node, Expr::Int(2));
            }
            other => panic!("expected Record, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── index expression ──────────────────────────────────────────

#[test]
fn test_index_access() {
    let prog = parse("arr[0]");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Index { object, index } => {
                assert!(matches!(object.node, Expr::Ident(ref n) if n == "arr"));
                assert_eq!(index.node, Expr::Int(0));
            }
            other => panic!("expected Index, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_index_access_expression() {
    let prog = parse("arr[i + 1]");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Index { object, index } => {
                assert!(matches!(object.node, Expr::Ident(ref n) if n == "arr"));
                assert!(matches!(index.node, Expr::Binary { op: BinaryOp::Add, .. }));
            }
            other => panic!("expected Index, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── range ─────────────────────────────────────────────────────

#[test]
fn test_range_exclusive() {
    let prog = parse("1..10");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Range {
                start,
                end,
                inclusive,
            } => {
                assert_eq!(start.node, Expr::Int(1));
                assert_eq!(end.node, Expr::Int(10));
                assert!(!inclusive);
            }
            other => panic!("expected Range, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_range_inclusive() {
    let prog = parse("1..=10");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Range { inclusive, .. } => {
                assert!(*inclusive);
            }
            other => panic!("expected Range, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── bio literals ──────────────────────────────────────────────

#[test]
fn test_dna_literal() {
    let prog = parse(r#"dna"ATCG""#);
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::DnaLit(seq) => assert_eq!(seq, "ATCG"),
            other => panic!("expected DnaLit, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_rna_literal() {
    let prog = parse(r#"rna"AUCG""#);
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::RnaLit(seq) => assert_eq!(seq, "AUCG"),
            other => panic!("expected RnaLit, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_protein_literal() {
    let prog = parse(r#"protein"MKWVL""#);
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::ProteinLit(seq) => assert_eq!(seq, "MKWVL"),
            other => panic!("expected ProteinLit, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── nested blocks ─────────────────────────────────────────────

#[test]
fn test_nested_blocks() {
    let prog = parse("{ { x } }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Block(stmts) => {
                assert_eq!(stmts.len(), 1);
                match &stmts[0].node {
                    Stmt::Expr(inner) => {
                        assert!(matches!(inner.node, Expr::Block(_)));
                    }
                    other => panic!("expected inner Block, got {other:?}"),
                }
            }
            other => panic!("expected Block, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── empty block ───────────────────────────────────────────────

#[test]
fn test_empty_block() {
    let prog = parse("{ }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Block(stmts) => {
                assert!(stmts.is_empty());
            }
            other => panic!("expected Block, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── empty list ────────────────────────────────────────────────

#[test]
fn test_empty_list() {
    let prog = parse("[]");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::List(items) => assert!(items.is_empty()),
            other => panic!("expected List, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── empty function body ───────────────────────────────────────

#[test]
fn test_empty_function_body() {
    let prog = parse("fn noop() { }");
    match &prog.stmts[0].node {
        Stmt::Fn { name, body, .. } => {
            assert_eq!(name, "noop");
            assert!(body.is_empty());
        }
        other => panic!("expected Fn, got {other:?}"),
    }
}

// ── multiple statements separated by newlines ─────────────────

#[test]
fn test_multiple_statements() {
    let prog = parse("let x = 1\nlet y = 2\nlet z = 3");
    assert_eq!(prog.stmts.len(), 3);
    for stmt in &prog.stmts {
        assert!(matches!(stmt.node, Stmt::Let { .. }));
    }
}

// ── operator precedence ───────────────────────────────────────

#[test]
fn test_mul_higher_than_add() {
    // 2 + 3 * 4 => 2 + (3 * 4)
    let prog = parse("2 + 3 * 4");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Binary { op, left, right } => {
                assert_eq!(*op, BinaryOp::Add);
                assert_eq!(left.node, Expr::Int(2));
                match &right.node {
                    Expr::Binary { op, .. } => assert_eq!(*op, BinaryOp::Mul),
                    other => panic!("expected Mul, got {other:?}"),
                }
            }
            other => panic!("expected Binary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_comparison_lower_than_add() {
    // a + b > c => (a + b) > c
    let prog = parse("a + b > c");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Binary { op, left, .. } => {
                assert_eq!(*op, BinaryOp::Gt);
                match &left.node {
                    Expr::Binary { op, .. } => assert_eq!(*op, BinaryOp::Add),
                    other => panic!("expected Add, got {other:?}"),
                }
            }
            other => panic!("expected Binary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_logical_lower_than_comparison() {
    // a > 1 && b < 2 => (a > 1) && (b < 2)
    let prog = parse("a > 1 && b < 2");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Binary { op, left, right } => {
                assert_eq!(*op, BinaryOp::And);
                assert!(matches!(left.node, Expr::Binary { op: BinaryOp::Gt, .. }));
                assert!(matches!(right.node, Expr::Binary { op: BinaryOp::Lt, .. }));
            }
            other => panic!("expected Binary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_or_lower_than_and() {
    // a && b || c => (a && b) || c
    let prog = parse("a && b || c");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Binary { op, left, .. } => {
                assert_eq!(*op, BinaryOp::Or);
                match &left.node {
                    Expr::Binary { op, .. } => assert_eq!(*op, BinaryOp::And),
                    other => panic!("expected And, got {other:?}"),
                }
            }
            other => panic!("expected Binary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_sub_and_div() {
    let prog = parse("10 - 3 / 2");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Binary { op, right, .. } => {
                assert_eq!(*op, BinaryOp::Sub);
                assert!(matches!(right.node, Expr::Binary { op: BinaryOp::Div, .. }));
            }
            other => panic!("expected Binary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_mod_operator() {
    let prog = parse("x % 2");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Binary { op, .. } => assert_eq!(*op, BinaryOp::Mod),
            other => panic!("expected Binary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── parenthesized expressions ─────────────────────────────────

#[test]
fn test_parenthesized_expression() {
    // (a + b) * c => Mul(Add(a, b), c)
    let prog = parse("(a + b) * c");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Binary { op, left, .. } => {
                assert_eq!(*op, BinaryOp::Mul);
                match &left.node {
                    Expr::Binary { op, .. } => assert_eq!(*op, BinaryOp::Add),
                    other => panic!("expected inner Add, got {other:?}"),
                }
            }
            other => panic!("expected Binary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── lambda variants ───────────────────────────────────────────

#[test]
fn test_lambda_multiple_params() {
    let prog = parse("|a, b| a + b");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Lambda { params, body } => {
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "a");
                assert_eq!(params[1].name, "b");
                assert!(matches!(body.node, Expr::Binary { op: BinaryOp::Add, .. }));
            }
            other => panic!("expected Lambda, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

/// `||` is lexed as `TokenKind::Or`, so zero-param lambdas are not supported.
/// This test verifies the parser rejects `|| 42`.
#[test]
fn test_lambda_no_params_unsupported() {
    assert!(parse_has_errors("|| 42"));
}

// ── default parameter values ──────────────────────────────────

#[test]
fn test_function_default_params() {
    let prog = parse("fn greet(name = \"world\") { name }");
    match &prog.stmts[0].node {
        Stmt::Fn { params, .. } => {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "name");
            assert!(params[0].default.is_some());
            match &params[0].default.as_ref().unwrap().node {
                Expr::Str(s) => assert_eq!(s, "world"),
                other => panic!("expected Str default, got {other:?}"),
            }
        }
        other => panic!("expected Fn, got {other:?}"),
    }
}

// ── rest/spread args ──────────────────────────────────────────

#[test]
fn test_rest_parameter() {
    let prog = parse("fn sum(...args) { args }");
    match &prog.stmts[0].node {
        Stmt::Fn { params, .. } => {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "args");
            assert!(params[0].rest);
        }
        other => panic!("expected Fn, got {other:?}"),
    }
}

#[test]
fn test_spread_arg() {
    let prog = parse("f(...items)");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 1);
                assert!(args[0].spread);
            }
            other => panic!("expected Call, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── method call syntax ────────────────────────────────────────

#[test]
fn test_method_call() {
    let prog = parse("obj.method(arg)");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Call { callee, args } => {
                match &callee.node {
                    Expr::Field { object, field, .. } => {
                        assert!(matches!(object.node, Expr::Ident(ref n) if n == "obj"));
                        assert_eq!(field, "method");
                    }
                    other => panic!("expected Field, got {other:?}"),
                }
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected Call, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── chained field access ──────────────────────────────────────

#[test]
fn test_chained_field_access() {
    let prog = parse("a.b.c");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Field { object, field, .. } => {
                assert_eq!(field, "c");
                match &object.node {
                    Expr::Field {
                        object: inner,
                        field: f2,
                        ..
                    } => {
                        assert_eq!(f2, "b");
                        assert!(matches!(inner.node, Expr::Ident(ref n) if n == "a"));
                    }
                    other => panic!("expected inner Field, got {other:?}"),
                }
            }
            other => panic!("expected Field, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── additional language constructs ────────────────────────────

#[test]
fn test_let_with_type_annotation() {
    let prog = parse("let x: Int = 42");
    match &prog.stmts[0].node {
        Stmt::Let {
            name,
            type_ann,
            value,
        } => {
            assert_eq!(name, "x");
            assert_eq!(type_ann.as_ref().unwrap().name, "Int");
            assert_eq!(value.node, Expr::Int(42));
        }
        other => panic!("expected Let, got {other:?}"),
    }
}

#[test]
fn test_function_return_type() {
    let prog = parse("fn add(a: Int, b: Int) -> Int { a + b }");
    match &prog.stmts[0].node {
        Stmt::Fn {
            return_type, params, ..
        } => {
            assert_eq!(return_type.as_ref().unwrap().name, "Int");
            assert_eq!(params[0].type_ann.as_ref().unwrap().name, "Int");
        }
        other => panic!("expected Fn, got {other:?}"),
    }
}

#[test]
fn test_boolean_literals() {
    let prog = parse("true");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => assert_eq!(expr.node, Expr::Bool(true)),
        other => panic!("expected Expr, got {other:?}"),
    }

    let prog = parse("false");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => assert_eq!(expr.node, Expr::Bool(false)),
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_nil_literal() {
    let prog = parse("nil");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => assert_eq!(expr.node, Expr::Nil),
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_float_literal() {
    let prog = parse("3.14");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Float(f) => assert!((*f - 3.14).abs() < f64::EPSILON),
            other => panic!("expected Float, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_string_literal() {
    let prog = parse(r#""hello""#);
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Str(s) => assert_eq!(s, "hello"),
            other => panic!("expected Str, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_variable_reassignment() {
    let prog = parse("x = 10");
    match &prog.stmts[0].node {
        Stmt::Assign { name, value } => {
            assert_eq!(name, "x");
            assert_eq!(value.node, Expr::Int(10));
        }
        other => panic!("expected Assign, got {other:?}"),
    }
}

#[test]
fn test_compound_assign_all_ops() {
    for (input, expected_op) in [
        ("x += 1", BinaryOp::Add),
        ("x -= 1", BinaryOp::Sub),
        ("x *= 2", BinaryOp::Mul),
        ("x /= 3", BinaryOp::Div),
    ] {
        let prog = parse(input);
        match &prog.stmts[0].node {
            Stmt::Assign { value, .. } => match &value.node {
                Expr::Binary { op, .. } => assert_eq!(*op, expected_op, "failed for {input}"),
                other => panic!("expected Binary for {input}, got {other:?}"),
            },
            other => panic!("expected Assign for {input}, got {other:?}"),
        }
    }
}

#[test]
fn test_equality_operators() {
    let prog = parse("a == b");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Binary { op, .. } => assert_eq!(*op, BinaryOp::Eq),
            other => panic!("expected Binary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }

    let prog = parse("a != b");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Binary { op, .. } => assert_eq!(*op, BinaryOp::Neq),
            other => panic!("expected Binary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_comparison_operators() {
    for (input, expected_op) in [
        ("a < b", BinaryOp::Lt),
        ("a > b", BinaryOp::Gt),
        ("a <= b", BinaryOp::Le),
        ("a >= b", BinaryOp::Ge),
    ] {
        let prog = parse(input);
        match &prog.stmts[0].node {
            Stmt::Expr(expr) => match &expr.node {
                Expr::Binary { op, .. } => {
                    assert_eq!(*op, expected_op, "failed for {input}");
                }
                other => panic!("expected Binary for {input}, got {other:?}"),
            },
            other => panic!("expected Expr for {input}, got {other:?}"),
        }
    }
}

#[test]
fn test_destruct_list_with_rest() {
    let prog = parse("let [a, b, ...rest] = items");
    match &prog.stmts[0].node {
        Stmt::DestructLet { pattern, .. } => match pattern {
            DestructPattern::ListWithRest {
                elements,
                rest_name,
            } => {
                assert_eq!(elements, &["a", "b"]);
                assert_eq!(rest_name, "rest");
            }
            other => panic!("expected ListWithRest, got {other:?}"),
        },
        other => panic!("expected DestructLet, got {other:?}"),
    }
}

#[test]
fn test_enum_definition() {
    let prog = parse("enum Color {\n  Red,\n  Green,\n  Blue\n}");
    match &prog.stmts[0].node {
        Stmt::Enum { name, variants } => {
            assert_eq!(name, "Color");
            assert_eq!(variants.len(), 3);
            assert_eq!(variants[0].name, "Red");
            assert!(variants[0].fields.is_empty());
        }
        other => panic!("expected Enum, got {other:?}"),
    }
}

#[test]
fn test_enum_with_fields() {
    let prog = parse("enum Option {\n  Some(value),\n  None\n}");
    match &prog.stmts[0].node {
        Stmt::Enum { name, variants } => {
            assert_eq!(name, "Option");
            assert_eq!(variants[0].name, "Some");
            assert_eq!(variants[0].fields, &["value"]);
            assert_eq!(variants[1].name, "None");
            assert!(variants[1].fields.is_empty());
        }
        other => panic!("expected Enum, got {other:?}"),
    }
}

#[test]
fn test_trait_definition() {
    let prog = parse("trait Printable {\n  fn print(self)\n  fn repr(self)\n}");
    match &prog.stmts[0].node {
        Stmt::Trait { name, methods } => {
            assert_eq!(name, "Printable");
            assert_eq!(methods.len(), 2);
            assert_eq!(methods[0].name, "print");
            assert_eq!(methods[1].name, "repr");
        }
        other => panic!("expected Trait, got {other:?}"),
    }
}

#[test]
fn test_pipeline_statement() {
    let prog = parse(r#"pipeline "my_pipe" { let x = 1 }"#);
    match &prog.stmts[0].node {
        Stmt::Pipeline { name, params, body } => {
            assert_eq!(name, "my_pipe");
            assert!(params.is_empty());
            assert_eq!(body.len(), 1);
        }
        other => panic!("expected Pipeline, got {other:?}"),
    }
}

#[test]
fn test_pipeline_with_params() {
    let prog = parse(r#"pipeline align(sample, reference) { let x = 1 }"#);
    match &prog.stmts[0].node {
        Stmt::Pipeline { name, params, body } => {
            assert_eq!(name, "align");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "sample");
            assert_eq!(params[1].name, "reference");
            assert_eq!(body.len(), 1);
        }
        other => panic!("expected Pipeline, got {other:?}"),
    }
}

#[test]
fn test_pipeline_ident_name() {
    let prog = parse("pipeline my_pipe { let x = 1 }");
    match &prog.stmts[0].node {
        Stmt::Pipeline { name, params, body } => {
            assert_eq!(name, "my_pipe");
            assert!(params.is_empty());
            assert_eq!(body.len(), 1);
        }
        other => panic!("expected Pipeline, got {other:?}"),
    }
}

#[test]
fn test_assert_simple() {
    let prog = parse("assert x > 0");
    match &prog.stmts[0].node {
        Stmt::Assert {
            condition, message, ..
        } => {
            assert!(matches!(condition.node, Expr::Binary { .. }));
            assert!(message.is_none());
        }
        other => panic!("expected Assert, got {other:?}"),
    }
}

#[test]
fn test_assert_with_message() {
    let prog = parse(r#"assert x > 0, "x must be positive""#);
    match &prog.stmts[0].node {
        Stmt::Assert { message, .. } => {
            assert!(message.is_some());
        }
        other => panic!("expected Assert, got {other:?}"),
    }
}

#[test]
fn test_try_catch_no_var() {
    let prog = parse("try { risky() } catch { fallback() }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::TryCatch { error_var, .. } => {
                assert!(error_var.is_none());
            }
            other => panic!("expected TryCatch, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_optional_field_access() {
    let prog = parse("obj?.field");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Field {
                field, optional, ..
            } => {
                assert_eq!(field, "field");
                assert!(*optional);
            }
            other => panic!("expected Field, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_list_comprehension() {
    let prog = parse("[x * 2 for x in items]");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::ListComp {
                var, condition, ..
            } => {
                assert_eq!(var, "x");
                assert!(condition.is_none());
            }
            other => panic!("expected ListComp, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

/// List comprehension with `if` guard conflicts with ternary `if` parsing
/// because `parse_expr` on the iterable consumes `items if ...` as a ternary.
/// This test documents the current behavior: the iterable must be parenthesized
/// or the comprehension `if` is not used.
#[test]
fn test_list_comprehension_with_condition_conflict() {
    // Direct `if` after iterable triggers ternary parse, which fails
    assert!(parse_has_errors("[x for x in items if x > 0]"));
}

#[test]
fn test_const_statement() {
    let prog = parse("const PI = 3.14");
    match &prog.stmts[0].node {
        Stmt::Const { name, value, .. } => {
            assert_eq!(name, "PI");
            assert!(matches!(value.node, Expr::Float(_)));
        }
        other => panic!("expected Const, got {other:?}"),
    }
}

#[test]
fn test_formula_expression() {
    // `~` captures at Addition precedence (exclusive), so `~x + y` => `(~x) + y`
    let prog = parse("~x + y");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Binary { op, left, .. } => {
                assert_eq!(*op, BinaryOp::Add);
                assert!(matches!(left.node, Expr::Formula(_)));
            }
            other => panic!("expected Binary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }

    // Standalone formula
    let prog = parse("~x");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => {
            assert!(matches!(expr.node, Expr::Formula(_)));
        }
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_chained_comparison() {
    let prog = parse("a < b < c");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::ChainedCmp { operands, ops } => {
                assert_eq!(operands.len(), 3);
                assert_eq!(ops.len(), 2);
                assert_eq!(ops[0], BinaryOp::Lt);
                assert_eq!(ops[1], BinaryOp::Lt);
            }
            other => panic!("expected ChainedCmp, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_lambda_with_block_body() {
    let prog = parse("|x| { let y = x + 1\n y }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Lambda { params, body } => {
                assert_eq!(params.len(), 1);
                assert!(matches!(body.node, Expr::Block(_)));
            }
            other => panic!("expected Lambda, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_lambda_rest_param() {
    let prog = parse("|...args| args");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Lambda { params, .. } => {
                assert_eq!(params.len(), 1);
                assert!(params[0].rest);
                assert_eq!(params[0].name, "args");
            }
            other => panic!("expected Lambda, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_set_literal() {
    let prog = parse("#{1, 2, 3}");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::SetLiteral(items) => {
                assert_eq!(items.len(), 3);
            }
            other => panic!("expected SetLiteral, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_qual_literal() {
    let prog = parse(r#"qual"FFFFFFFF""#);
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::QualLit(q) => assert_eq!(q, "FFFFFFFF"),
            other => panic!("expected QualLit, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_yield_statement() {
    let prog = parse("fn* gen() { yield 1 }");
    match &prog.stmts[0].node {
        Stmt::Fn { body, is_generator, .. } => {
            assert!(*is_generator);
            assert!(matches!(body[0].node, Stmt::Yield(_)));
        }
        other => panic!("expected Fn, got {other:?}"),
    }
}

// ── empty input ───────────────────────────────────────────────

#[test]
fn test_empty_input() {
    let prog = parse("");
    assert!(prog.stmts.is_empty());
}

#[test]
fn test_whitespace_only() {
    let prog = parse("   ");
    assert!(prog.stmts.is_empty());
}

// ── error cases ───────────────────────────────────────────────

#[test]
fn test_error_missing_closing_paren() {
    assert!(parse_has_errors("f(a, b"));
}

#[test]
fn test_error_missing_closing_brace() {
    assert!(parse_has_errors("if true { x"));
}

#[test]
fn test_error_unexpected_token() {
    assert!(parse_has_errors(")"));
}

#[test]
fn test_error_incomplete_let() {
    assert!(parse_has_errors("let"));
}

#[test]
fn test_error_missing_equals_in_let() {
    assert!(parse_has_errors("let x 42"));
}

// ── struct literal ────────────────────────────────────────────

#[test]
fn test_struct_literal() {
    let prog = parse("Point { x: 1, y: 2 }");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::StructLit { name, fields } => {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "x");
                assert_eq!(fields[1].0, "y");
            }
            other => panic!("expected StructLit, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

// ── complex combinations ──────────────────────────────────────

#[test]
fn test_method_chain_with_pipe() {
    let prog = parse("data.filter() |> map()");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => {
            assert!(matches!(expr.node, Expr::Pipe { .. }));
        }
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_pipe_into_binding() {
    let prog = parse("[1, 2, 3] |> len() |> into count");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::PipeInto { value, name } => {
                assert_eq!(name, "count");
                assert!(matches!(value.node, Expr::Pipe { .. }));
            }
            other => panic!("expected PipeInto, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_nested_index_access() {
    let prog = parse("matrix[0][1]");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Index { object, index } => {
                assert_eq!(index.node, Expr::Int(1));
                assert!(matches!(object.node, Expr::Index { .. }));
            }
            other => panic!("expected Index, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_call_on_field_result() {
    let prog = parse("obj.method(1).other(2)");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Call { callee, args } => {
                assert_eq!(args.len(), 1);
                match &callee.node {
                    Expr::Field { field, object, .. } => {
                        assert_eq!(field, "other");
                        assert!(matches!(object.node, Expr::Call { .. }));
                    }
                    other => panic!("expected Field, got {other:?}"),
                }
            }
            other => panic!("expected Call, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_complex_for_loop_body() {
    let prog = parse("for x in 1..10 {\n  if x > 5 {\n    break\n  }\n}");
    match &prog.stmts[0].node {
        Stmt::For { iter, body, .. } => {
            assert!(matches!(iter.node, Expr::Range { .. }));
            assert_eq!(body.len(), 1);
        }
        other => panic!("expected For, got {other:?}"),
    }
}

#[test]
fn test_null_coalesce_chain() {
    let prog = parse("a ?? b ?? c");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => {
            assert!(matches!(expr.node, Expr::NullCoalesce { .. }));
        }
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_multiple_named_args() {
    let prog = parse("f(x: 1, y: 2, z: 3)");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Call { args, .. } => {
                assert_eq!(args.len(), 3);
                assert_eq!(args[0].name.as_deref(), Some("x"));
                assert_eq!(args[1].name.as_deref(), Some("y"));
                assert_eq!(args[2].name.as_deref(), Some("z"));
            }
            other => panic!("expected Call, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_deeply_nested_expression() {
    let prog = parse("((1 + 2) * (3 - 4)) / 5");
    match &prog.stmts[0].node {
        Stmt::Expr(expr) => match &expr.node {
            Expr::Binary { op, .. } => {
                assert_eq!(*op, BinaryOp::Div);
            }
            other => panic!("expected Binary, got {other:?}"),
        },
        other => panic!("expected Expr, got {other:?}"),
    }
}

#[test]
fn test_with_statement() {
    let prog = parse("with ctx { do_something() }");
    match &prog.stmts[0].node {
        Stmt::With { body, .. } => {
            assert_eq!(body.len(), 1);
        }
        other => panic!("expected With, got {other:?}"),
    }
}

// ============================================================================
// given { ... } — regression tests for OOM parser bug
// ============================================================================

#[test]
fn test_given_newline_separated_arms() {
    // Was OOM: parser broke after first arm due to else{break} after comma check
    let _prog = parse(r#"
given {
    x < 0     => "negative"
    x == 0    => "zero"
    x < 100   => "small"
    otherwise => "large"
}
"#);
}

#[test]
fn test_given_comma_separated_arms() {
    let _prog = parse(r#"
given {
    x < 0 => "negative",
    x == 0 => "zero",
    otherwise => "large",
}
"#);
}

#[test]
fn test_given_single_arm_no_comma() {
    let _prog = parse("given { true => 42 }");
}

#[test]
fn test_given_otherwise_only() {
    let _prog = parse("given { otherwise => 99 }");
}

#[test]
fn test_given_empty() {
    let _prog = parse("given {}");
}

#[test]
fn test_given_with_complex_exprs() {
    let _prog = parse(r#"
given {
    a > 10 && b < 20 => call_fn(a, b)
    c + d == 42      => [1, 2, 3]
    otherwise         => {x: 1, y: 2}
}
"#);
}

#[test]
fn test_given_nested_in_for() {
    let _prog = parse(r#"
for v in items {
    let label = given {
        is_snp(v) => "snp"
        is_indel(v) => "indel"
        otherwise => "other"
    }
    print(label)
}
"#);
}

#[test]
fn test_given_nested_given() {
    let _prog = parse(r#"
given {
    x == 1 => given {
        y == 1 => "a"
        y == 2 => "b"
        otherwise => "c"
    }
    otherwise => "d"
}
"#);
}

#[test]
fn test_given_with_block_bodies() {
    let _prog = parse(r#"
given {
    x > 10 => {
        let a = x * 2
        a + 1
    }
    otherwise => {
        let b = x - 1
        b
    }
}
"#);
}

#[test]
fn test_given_five_arms_newline() {
    // Ensure 5+ arms with newlines work (was breaking after 1st arm)
    let _prog = parse(r#"
given {
    a => 1
    b => 2
    c => 3
    d => 4
    e => 5
    otherwise => 0
}
"#);
}

// ============================================================================
// Multi-line pipes — regression test for newline-before-pipe
// ============================================================================

#[test]
fn test_multiline_pipe_let() {
    let _prog = parse(r#"
let result = data
    |> filter(|n| n > 3)
    |> map(|n| n * 2)
"#);
}

#[test]
fn test_multiline_pipe_bare_expr() {
    let _prog = parse(r#"
items
    |> filter(|x| x > 0)
    |> map(|x| x * 2)
    |> reduce(0, |a, b| a + b)
"#);
}

#[test]
fn test_multiline_pipe_after_call() {
    let _prog = parse(r#"
get_data()
    |> filter(|x| x.valid)
    |> each |x| process(x)
"#);
}

// ============================================================================
// Trailing lambda in pipes
// ============================================================================

#[test]
fn test_pipe_trailing_lambda_each() {
    let _prog = parse(r#"
items |> each |x| print(x)
"#);
}

#[test]
fn test_pipe_trailing_lambda_map() {
    let _prog = parse(r#"
items |> map |x| x * 2
"#);
}

#[test]
fn test_pipe_trailing_lambda_filter() {
    let _prog = parse(r#"
items |> filter |x| x > 0
"#);
}

#[test]
fn test_pipe_trailing_lambda_last_in_chain() {
    // Trailing lambda works on the last pipe; earlier pipes need parens
    let _prog = parse(r#"
data
    |> filter(|x| x > 0)
    |> map |x| x * 2
"#);
}

// ============================================================================
// String-key records (was parser OOM trigger)
// ============================================================================

#[test]
fn test_string_key_record_simple() {
    let _prog = parse(r#"
let r = {"key": "value", "num": 42}
"#);
}

#[test]
fn test_string_key_record_in_condition() {
    let _prog = parse(r#"
if r.key == "value" { "yes" } else { "no" }
"#);
}

// ============================================================================
// if...then (was parser bug)
// ============================================================================

#[test]
fn test_if_then_simple() {
    let _prog = parse(r#"
if x > 0 then print("positive")
"#);
}

#[test]
fn test_if_then_else() {
    let _prog = parse(r#"
if x > 0 then "positive" else "non-positive"
"#);
}
