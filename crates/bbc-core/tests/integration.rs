use bbc_core::env::Env;
use bbc_core::eval::Evaluator;

fn eval(input: &str) -> String {
    let evaluator = Evaluator::new();
    let mut env = Env::new();
    bbc_core::evaluate_and_format(input, &mut env, &evaluator).unwrap()
}

fn eval_err(input: &str) -> String {
    let evaluator = Evaluator::new();
    let mut env = Env::new();
    bbc_core::evaluate_and_format(input, &mut env, &evaluator)
        .unwrap_err()
        .to_string()
}

fn eval_with_env(inputs: &[&str]) -> Vec<String> {
    let evaluator = Evaluator::new();
    let mut env = Env::new();
    inputs
        .iter()
        .map(|input| {
            bbc_core::evaluate_and_format(input, &mut env, &evaluator)
                .unwrap_or_else(|e| format!("error: {}", e))
        })
        .collect()
}

// --- Basic arithmetic ---

#[test]
fn integer_arithmetic() {
    assert_eq!(eval("2 + 3"), "5");
    assert_eq!(eval("10 - 7"), "3");
    assert_eq!(eval("6 * 7"), "42");
    assert_eq!(eval("100 / 4"), "25");
    assert_eq!(eval("17 % 5"), "2");
}

#[test]
fn exact_rational_arithmetic() {
    assert_eq!(eval("1/3 * 3"), "1");
    assert_eq!(eval("1/7 + 1/7 + 1/7 + 1/7 + 1/7 + 1/7 + 1/7"), "1");
}

#[test]
fn operator_precedence() {
    assert_eq!(eval("2 + 3 * 4"), "14");
    assert_eq!(eval("(2 + 3) * 4"), "20");
    assert_eq!(eval("2 ^ 3 ^ 2"), "512");
}

#[test]
fn negative_numbers() {
    assert_eq!(eval("-5"), "-5");
    assert_eq!(eval("-5 + 3"), "-2");
    assert_eq!(eval("3 * -2"), "-6");
}

#[test]
fn division_by_zero() {
    let err = eval_err("1 / 0");
    assert!(err.contains("division by zero"));
}

// --- Base conversion ---

#[test]
fn hex_literals() {
    assert_eq!(eval("16xFF"), "255");
    assert_eq!(eval("16xFF + 1"), "256");
}

#[test]
fn binary_literals() {
    assert_eq!(eval("2x1010"), "10");
    assert_eq!(eval("2x1111"), "15");
}

#[test]
fn octal_literals() {
    assert_eq!(eval("8x77"), "63");
}

#[test]
fn mixed_base_arithmetic() {
    assert_eq!(eval("16xFF + 2x1010"), "265");
}

#[test]
fn obase_hex() {
    let results = eval_with_env(&["obase = 16", "255"]);
    assert_eq!(results[1], "16xFF");
}

#[test]
fn obase_binary() {
    let results = eval_with_env(&["obase = 2", "10"]);
    assert_eq!(results[1], "2x1010");
}

// --- Unit system ---

#[test]
fn force_calculation() {
    assert_eq!(eval("5 [kg] * 9.8 [m*s^-2]"), "49 [N]");
}

#[test]
fn unit_conversion() {
    let result = eval("100 [km] -> [mi]");
    assert!(result.contains("[mi]"));
    assert!(result.contains("62.137"));
}

#[test]
fn dimension_mismatch() {
    let err = eval_err("9.8 [m*s^-2] + 1 [K]");
    assert!(err.contains("dimension mismatch"));
}

#[test]
fn unit_addition() {
    let result = eval("3 [km] + 500 [m]");
    assert!(result.contains("3500"));
}

// --- Built-in functions ---

#[test]
fn trig_functions() {
    assert_eq!(eval("sin(0)"), "0");
    assert_eq!(eval("cos(0)"), "1");
}

#[test]
fn sqrt_function() {
    assert_eq!(eval("sqrt(144)"), "12");
}

#[test]
fn abs_function() {
    assert_eq!(eval("abs(-42)"), "42");
    assert_eq!(eval("abs(42)"), "42");
}

#[test]
fn floor_ceil_round() {
    assert_eq!(eval("floor(3.7)"), "3");
    assert_eq!(eval("ceil(3.2)"), "4");
    assert_eq!(eval("round(3.5)"), "4");
}

#[test]
fn min_max() {
    assert_eq!(eval("min(3, 7)"), "3");
    assert_eq!(eval("max(3, 7)"), "7");
}

#[test]
fn ln_exp() {
    assert_eq!(eval("exp(0)"), "1");
    assert_eq!(eval("ln(1)"), "0");
}

// --- Constants ---

#[test]
fn pi_constant() {
    let result = eval("pi");
    assert!(result.starts_with("3.14159265"));
}

#[test]
fn e_constant() {
    let result = eval("e");
    assert!(result.starts_with("2.71828182"));
}

// --- Variables ---

#[test]
fn variable_assignment() {
    let results = eval_with_env(&["x = 42", "x * 2"]);
    assert_eq!(results[0], "42");
    assert_eq!(results[1], "84");
}

#[test]
fn variable_with_units() {
    let results = eval_with_env(&["g = 9.8 [m*s^-2]", "5 [kg] * g"]);
    assert_eq!(results[1], "49 [N]");
}

// --- Bitwise operations ---

#[test]
fn bitwise_and() {
    assert_eq!(eval("16xFF & 16x0F"), "15");
}

#[test]
fn bitwise_or() {
    assert_eq!(eval("16xFF | 16x100"), "511");
}

#[test]
fn bitwise_xor() {
    assert_eq!(eval("16xFF ^^ 16x0F"), "240");
}

#[test]
fn bitwise_not() {
    assert_eq!(eval("~0"), "-1");
}

#[test]
fn bitwise_shift() {
    assert_eq!(eval("1 << 8"), "256");
    assert_eq!(eval("256 >> 4"), "16");
}

// --- Boolean operations ---

#[test]
fn boolean_ops() {
    assert_eq!(eval("true && false"), "false");
    assert_eq!(eval("true || false"), "true");
    assert_eq!(eval("!true"), "false");
}

// --- Comparisons ---

#[test]
fn comparisons() {
    assert_eq!(eval("3 < 5"), "true");
    assert_eq!(eval("5 < 3"), "false");
    assert_eq!(eval("3 == 3"), "true");
    assert_eq!(eval("3 != 4"), "true");
}

// --- Scale setting ---

#[test]
fn scale_setting() {
    let results = eval_with_env(&["scale = 6", "1/3"]);
    assert_eq!(results[1], "0.333333");
}

// --- Error handling ---

#[test]
fn unknown_variable_error() {
    let err = eval_err("xyz");
    assert!(err.contains("unknown variable"));
}

#[test]
fn unknown_function_error() {
    let err = eval_err("foo(1)");
    assert!(err.contains("unknown function"));
}

#[test]
fn unknown_unit_error() {
    let err = eval_err("5 [qux]");
    assert!(err.contains("unknown unit"));
}

// --- Arrow precedence ---

#[test]
fn arrow_lowest_precedence() {
    // -> should bind after the full expression: (10 [m/s] + 45 [km/min]) -> [mph]
    let result = eval("10 [m/s] + 45 [km/min] -> [mph]");
    assert!(result.contains("[mph]"));
    assert!(result.contains("1700"));
}
