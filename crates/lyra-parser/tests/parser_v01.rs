use lyra_ast::{BinaryOperator, Expression, Item, Module, Statement, UnaryOperator};
use lyra_diagnostics::Diagnostic;
use lyra_lexer::tokenize;
use lyra_parser::parse;

fn parse_source(source: &str) -> (Module, Vec<Diagnostic>) {
    let lexed = tokenize(source);
    assert!(lexed.diagnostics.is_empty());
    parse(&lexed.tokens)
}

fn return_expression(source: &str) -> Expression {
    let (module, diagnostics) = parse_source(source);
    assert!(diagnostics.is_empty());
    let Item::Function(function) = &module.items[0];
    let Statement::Return {
        value: Some(expression),
        ..
    } = &function.body.statements[0]
    else {
        panic!("expected return expression");
    };
    expression.clone()
}

#[test]
fn subtraction_is_left_associative() {
    let expression = return_expression("fn main() { return 10 - 3 - 2; }");
    let Expression::Binary {
        left,
        operator: BinaryOperator::Subtract,
        right,
        ..
    } = expression
    else {
        panic!("expected subtraction at expression root");
    };
    assert!(matches!(*right, Expression::Integer(2, _)));
    assert!(matches!(
        *left,
        Expression::Binary {
            operator: BinaryOperator::Subtract,
            ..
        }
    ));
}

#[test]
fn parentheses_override_precedence() {
    let expression = return_expression("fn main() { return (1 + 2) * 3; }");
    let Expression::Binary {
        left,
        operator: BinaryOperator::Multiply,
        ..
    } = expression
    else {
        panic!("expected multiplication at expression root");
    };
    assert!(matches!(
        *left,
        Expression::Binary {
            operator: BinaryOperator::Add,
            ..
        }
    ));
}

#[test]
fn unary_operators_bind_tighter_than_binary_operators() {
    let expression = return_expression("fn main() { return -speed * 2; }");
    let Expression::Binary {
        left,
        operator: BinaryOperator::Multiply,
        ..
    } = expression
    else {
        panic!("expected multiplication at expression root");
    };
    assert!(matches!(
        *left,
        Expression::Unary {
            operator: UnaryOperator::Negate,
            ..
        }
    ));
}

#[test]
fn parses_bare_return() {
    let (module, diagnostics) = parse_source("fn main() { return; }");
    assert!(diagnostics.is_empty());
    let Item::Function(function) = &module.items[0];
    assert!(matches!(
        function.body.statements[0],
        Statement::Return { value: None, .. }
    ));
}

#[test]
fn reports_missing_closing_parenthesis() {
    let (_, diagnostics) = parse_source("fn main() { return (1 + 2; }");
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("`)`")));
}

#[test]
fn reports_missing_function_body() {
    let (_, diagnostics) = parse_source("fn main()");
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("`{`")));
}

#[test]
fn parses_multiple_functions() {
    let (module, diagnostics) =
        parse_source("fn first() { return 1; } fn second() { return 2; }");
    assert!(diagnostics.is_empty());
    assert_eq!(module.items.len(), 2);
}
