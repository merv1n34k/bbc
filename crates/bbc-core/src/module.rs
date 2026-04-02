use crate::env::Env;
use crate::error::Error;
use crate::value::Value;

pub struct FnSignature {
    pub name: &'static str,
    pub min_args: usize,
    pub max_args: usize,
}

pub trait Module: Send + Sync {
    fn name(&self) -> &str;
    fn functions(&self) -> &[FnSignature];
    fn call(&self, name: &str, args: &[Value], env: &Env) -> Result<Value, Error>;
}
