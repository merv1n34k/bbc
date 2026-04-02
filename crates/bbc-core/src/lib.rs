pub mod ast;
pub mod dim;
pub mod env;
pub mod error;
pub mod eval;
pub mod format;
pub mod lexer;
pub mod module;
pub mod parser;
pub mod units;
pub mod value;

use env::Env;
use error::Error;
use eval::Evaluator;
use value::Value;

pub fn evaluate(input: &str, env: &mut Env, evaluator: &Evaluator) -> Result<Value, Error> {
    let expr = parser::parse(input)?;
    evaluator.eval(&expr, env)
}

pub fn evaluate_and_format(input: &str, env: &mut Env, evaluator: &Evaluator) -> Result<String, Error> {
    let val = evaluate(input, env, evaluator)?;
    let obase = env.get_obase();
    let scale = env.get_scale();
    Ok(format::format_value(&val, obase, scale, &evaluator.registry))
}
