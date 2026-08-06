//! UPGMA must emit real branch lengths.
//!
//! The implementation computed the node height (`half = min_d / 2.0`), formatted
//! the Newick label with a hardcoded ":0", and then discarded the height with
//! `let _ = half;` to silence the unused-variable warning. Every tree came back
//! with the correct topology and every branch length zero - fine if you only
//! ever look at the shape, useless to anything that reads distances.

use bl_core::value::Value;
use bl_lexer::Lexer;
use bl_parser::Parser;
use bl_runtime::Interpreter;

fn eval(code: &str) -> Value {
    let tokens = Lexer::new(code).tokenize().unwrap();
    let parsed = Parser::new(tokens).parse().unwrap();
    Interpreter::new().run(&parsed.program).unwrap()
}

/// Three tips whose distances make the expected heights easy to check by hand:
/// a and b are 2 apart, c is 6 from both.
const THREE_TIPS: &str = r#"
let d = table([
    { x: 0.0, y: 2.0, z: 6.0 },
    { x: 2.0, y: 0.0, z: 6.0 },
    { x: 6.0, y: 6.0, z: 0.0 },
])
upgma(["a", "b", "c"], d)
"#;

#[test]
fn upgma_emits_non_zero_branch_lengths() {
    let tree = format!("{}", eval(THREE_TIPS));
    assert!(
        !tree.contains(":0)") && !tree.contains(":0,") && !tree.contains(":0;"),
        "tree still has zero-length branches: {tree}"
    );
}

#[test]
fn upgma_heights_are_half_the_joining_distance() {
    let tree = format!("{}", eval(THREE_TIPS));
    // a and b join at distance 2, so each gets a branch of 1.
    assert!(
        tree.contains("a:1.000000") && tree.contains("b:1.000000"),
        "expected a and b at height 1.0, got: {tree}"
    );
    // c joins the (a,b) cluster at distance 6, so c's branch is 3.
    assert!(
        tree.contains("c:3.000000"),
        "expected c at height 3.0, got: {tree}"
    );
}

#[test]
fn upgma_is_ultrametric() {
    // UPGMA's defining property: every tip is the same distance from the root.
    // Here that means the (a,b) node contributes 3 - 1 = 2 above a and b.
    let tree = format!("{}", eval(THREE_TIPS));
    assert!(
        tree.contains("):2.000000"),
        "internal branch should be 2.0 so all tips sit at 3.0: {tree}"
    );
}

#[test]
fn upgma_topology_is_still_correct() {
    let tree = format!("{}", eval(THREE_TIPS));
    assert!(tree.contains("a:") && tree.contains("b:") && tree.contains("c:"));
    // a and b are closest, so they must be the inner pair.
    let a = tree.find('a').unwrap();
    let b = tree.find('b').unwrap();
    let c = tree.find('c').unwrap();
    assert!(a < b && b < c, "unexpected tip order in {tree}");
}
