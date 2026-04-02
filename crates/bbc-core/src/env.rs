use std::collections::{HashMap, HashSet};

use crate::error::Error;
use crate::module::Module;
use crate::value::Value;

pub struct Env {
    variables: HashMap<String, Value>,
    immutable: HashSet<String>,
    modules: Vec<Box<dyn Module>>,
    sigfig: bool,
}

impl Env {
    pub fn new() -> Self {
        let mut env = Env {
            variables: HashMap::new(),
            immutable: HashSet::new(),
            modules: Vec::new(),
            sigfig: false,
        };
        // Default settings
        env.variables
            .insert("scale".into(), Value::from_int(20));
        env.variables
            .insert("obase".into(), Value::from_int(10));
        env
    }

    pub fn get_var(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    pub fn set_var(&mut self, name: String, val: Value) -> Result<(), Error> {
        if self.immutable.contains(&name) {
            return Err(Error::TypeError {
                msg: format!("cannot reassign constant '{}'", name),
                span: None,
            });
        }
        self.variables.insert(name, val);
        Ok(())
    }

    /// Register an immutable constant. Cannot be reassigned.
    pub fn set_constant(&mut self, name: String, val: Value) {
        self.variables.insert(name.clone(), val);
        self.immutable.insert(name);
    }

    pub fn is_constant(&self, name: &str) -> bool {
        self.immutable.contains(name)
    }

    pub fn register_module(&mut self, module: Box<dyn Module>) {
        self.modules.push(module);
    }

    pub fn call_module_fn(&self, name: &str, args: &[Value]) -> Option<Result<Value, Error>> {
        for module in &self.modules {
            for sig in module.functions() {
                if sig.name == name {
                    return Some(module.call(name, args, self));
                }
            }
        }
        None
    }

    pub fn get_scale(&self) -> u32 {
        match self.variables.get("scale") {
            Some(Value::Quantity(q)) => {
                use malachite_base::num::conversion::traits::RoundingFrom;
                use malachite_base::rounding_modes::RoundingMode;
                let (val, _) = u32::rounding_from(&q.val, RoundingMode::Floor);
                val
            }
            _ => 20,
        }
    }

    pub fn sigfig_mode(&self) -> bool {
        self.sigfig
    }

    pub fn set_sigfig(&mut self, on: bool) {
        self.sigfig = on;
    }

    pub fn get_obase(&self) -> u32 {
        match self.variables.get("obase") {
            Some(Value::Quantity(q)) => {
                use malachite_base::num::conversion::traits::RoundingFrom;
                use malachite_base::rounding_modes::RoundingMode;
                let (val, _) = u32::rounding_from(&q.val, RoundingMode::Floor);
                if val < 2 || val > 36 {
                    10
                } else {
                    val
                }
            }
            _ => 10,
        }
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}
