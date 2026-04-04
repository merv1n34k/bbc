use std::collections::{HashMap, HashSet};

use crate::error::Error;
use crate::module::Module;
use crate::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum View {
    Scientific,
    Adjust,
    Strict,
}

impl View {
    pub fn parse(s: &str) -> Option<View> {
        match s {
            "scientific" => Some(View::Scientific),
            "adjust" => Some(View::Adjust),
            "strict" => Some(View::Strict),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            View::Scientific => "scientific",
            View::Adjust => "adjust",
            View::Strict => "strict",
        }
    }

    pub fn all() -> &'static [View] {
        &[View::Scientific, View::Adjust, View::Strict]
    }
}

#[derive(Debug, Clone)]
pub struct ViewSet {
    views: HashSet<View>,
}

impl ViewSet {
    pub fn new() -> Self {
        let mut views = HashSet::new();
        views.insert(View::Adjust);
        ViewSet { views }
    }

    pub fn has(&self, view: View) -> bool {
        self.views.contains(&view)
    }

    pub fn add(&mut self, view: View) {
        match view {
            View::Strict => {
                self.views.remove(&View::Adjust);
            }
            View::Adjust => {
                self.views.remove(&View::Strict);
            }
            _ => {}
        }
        self.views.insert(view);
    }

    pub fn remove(&mut self, view: View) {
        self.views.remove(&view);
    }

    pub fn list(&self) -> Vec<View> {
        let mut v: Vec<View> = self.views.iter().copied().collect();
        v.sort_by_key(|view| match view {
            View::Scientific => 0,
            View::Adjust => 1,
            View::Strict => 2,
        });
        v
    }
}

impl Default for ViewSet {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Env {
    variables: HashMap<String, Value>,
    immutable: HashSet<String>,
    modules: Vec<Box<dyn Module>>,
    views: ViewSet,
}

impl Env {
    pub fn new() -> Self {
        let mut env = Env {
            variables: HashMap::new(),
            immutable: HashSet::new(),
            modules: Vec::new(),
            views: ViewSet::new(),
        };
        env.variables
            .insert("scale".into(), Value::from_int(20));
        env.variables
            .insert("obase".into(), Value::from_int(10));
        env.variables
            .insert("sigfig".into(), Value::Bool(false));
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
        matches!(self.variables.get("sigfig"), Some(Value::Bool(true)))
    }

    pub fn set_sigfig(&mut self, on: bool) {
        self.variables.insert("sigfig".into(), Value::Bool(on));
    }

    pub fn strict_mode(&self) -> bool {
        self.views.has(View::Strict)
    }

    pub fn views(&self) -> &ViewSet {
        &self.views
    }

    pub fn views_mut(&mut self) -> &mut ViewSet {
        &mut self.views
    }

    pub fn get_obase(&self) -> u32 {
        match self.variables.get("obase") {
            Some(Value::Quantity(q)) => {
                use malachite_base::num::conversion::traits::RoundingFrom;
                use malachite_base::rounding_modes::RoundingMode;
                let (val, _) = u32::rounding_from(&q.val, RoundingMode::Floor);
                if !(2..=36).contains(&val) {
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
