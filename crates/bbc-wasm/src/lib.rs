use wasm_bindgen::prelude::*;

use bbc_core::env::Env;
use bbc_core::eval::Evaluator;

#[wasm_bindgen]
pub struct WasmEvaluator {
    env: Env,
    evaluator: Evaluator,
}

impl Default for WasmEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmEvaluator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let mut env = Env::new();
        let evaluator = Evaluator::new();
        bbc_core::register_constants(&mut env);
        env.register_module(Box::new(bbc_bitwise::BitwiseModule));
        WasmEvaluator { env, evaluator }
    }

    pub fn eval(&mut self, input: &str) -> String {
        match bbc_core::evaluate_and_format(input, &mut self.env, &mut self.evaluator) {
            Ok(result) => result,
            Err(e) => format!("error: {}", e),
        }
    }
}
