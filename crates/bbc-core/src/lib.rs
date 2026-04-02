pub mod ast;
pub mod dim;
pub mod env;
pub mod error;
pub mod eval;
pub mod format;
pub mod latex;
pub mod lexer;
pub mod module;
pub mod parser;
pub mod units;
pub mod value;

use malachite::Rational;

use env::Env;
use error::Error;
use eval::Evaluator;
use units::UnitRegistry;
use value::{Quantity, Value};

pub fn evaluate(input: &str, env: &mut Env, evaluator: &mut Evaluator) -> Result<Value, Error> {
    let processed = if input.contains('\\') {
        latex::preprocess_latex(input)
    } else {
        input.to_string()
    };
    let expr = parser::parse(&processed)?;
    evaluator.eval(&expr, env)
}

pub fn evaluate_and_format(input: &str, env: &mut Env, evaluator: &mut Evaluator) -> Result<String, Error> {
    let val = evaluate(input, env, evaluator)?;
    let scale = env.get_scale();
    if env.strict_mode() {
        Ok(format::format_value_strict(&val, scale))
    } else {
        let obase = env.get_obase();
        Ok(format::format_value(&val, obase, scale, &evaluator.registry))
    }
}

/// Load all constants from TOML data files and register them as immutable in Env.
pub fn register_constants(env: &mut Env) {
    for c in UnitRegistry::load_constants() {
        let val = Rational::try_from(c.value)
            .unwrap_or_else(|_| Rational::from(c.value as i64));
        let quantity = Value::Quantity(Quantity::new(val, c.dim));
        env.set_constant(c.name, quantity);
    }
}
