use bbc_core::env::Env;
use bbc_core::error::Error;
use bbc_core::module::{FnSignature, Module};
use bbc_core::value::Value;
use malachite_base::num::conversion::traits::{IsInteger, RoundingFrom};
use malachite_base::rounding_modes::RoundingMode;
use malachite_nz::integer::Integer;

pub struct BitwiseModule;

impl Module for BitwiseModule {
    fn name(&self) -> &str {
        "bitwise"
    }

    fn functions(&self) -> &[FnSignature] {
        &FUNCTIONS
    }

    fn call(&self, name: &str, args: &[Value], _env: &Env) -> Result<Value, Error> {
        match name {
            "popcount" => {
                let n = require_uint(args, "popcount")?;
                let count = n.checked_count_ones().unwrap_or(0);
                Ok(Value::from_int(count as i64))
            }
            "clz" => {
                let n = require_uint(args, "clz")?;
                let (val, _) = u32::rounding_from(
                    &malachite_q::Rational::from(n),
                    RoundingMode::Floor,
                );
                Ok(Value::from_int(val.leading_zeros() as i64))
            }
            "ctz" => {
                let n = require_uint(args, "ctz")?;
                let (val, _) = u64::rounding_from(
                    &malachite_q::Rational::from(n),
                    RoundingMode::Floor,
                );
                if val == 0 {
                    Ok(Value::from_int(64))
                } else {
                    Ok(Value::from_int(val.trailing_zeros() as i64))
                }
            }
            "rotl" => {
                if args.len() != 2 {
                    return Err(Error::InvalidArguments {
                        msg: "rotl requires 2 arguments (value, shift)".into(),
                        span: None,
                    });
                }
                let val = get_u32(&args[0], "rotl")?;
                let shift = get_u32(&args[1], "rotl")?;
                Ok(Value::from_int(val.rotate_left(shift) as i64))
            }
            "rotr" => {
                if args.len() != 2 {
                    return Err(Error::InvalidArguments {
                        msg: "rotr requires 2 arguments (value, shift)".into(),
                        span: None,
                    });
                }
                let val = get_u32(&args[0], "rotr")?;
                let shift = get_u32(&args[1], "rotr")?;
                Ok(Value::from_int(val.rotate_right(shift) as i64))
            }
            _ => Err(Error::UnknownFunction {
                name: name.to_string(),
                span: None,
            }),
        }
    }
}

static FUNCTIONS: [FnSignature; 5] = [
    FnSignature { name: "popcount", min_args: 1, max_args: 1 },
    FnSignature { name: "clz", min_args: 1, max_args: 1 },
    FnSignature { name: "ctz", min_args: 1, max_args: 1 },
    FnSignature { name: "rotl", min_args: 2, max_args: 2 },
    FnSignature { name: "rotr", min_args: 2, max_args: 2 },
];

fn require_uint(args: &[Value], name: &str) -> Result<Integer, Error> {
    if args.len() != 1 {
        return Err(Error::InvalidArguments {
            msg: format!("{} requires 1 argument", name),
            span: None,
        });
    }
    let q = args[0].as_quantity().ok_or_else(|| Error::TypeError {
        msg: format!("{} requires a number", name),
        span: None,
    })?;
    if !q.dim.is_dimensionless() {
        return Err(Error::TypeError {
            msg: format!("{} requires dimensionless integer", name),
            span: None,
        });
    }
    if !(&q.val).is_integer() {
        return Err(Error::TypeError {
            msg: format!("{} requires integer", name),
            span: None,
        });
    }
    let (n, _) = Integer::rounding_from(q.val.clone(), RoundingMode::Floor);
    Ok(n)
}

fn get_u32(val: &Value, name: &str) -> Result<u32, Error> {
    let q = val.as_quantity().ok_or_else(|| Error::TypeError {
        msg: format!("{} requires a number", name),
        span: None,
    })?;
    if !q.dim.is_dimensionless() || !(&q.val).is_integer() {
        return Err(Error::TypeError {
            msg: format!("{} requires dimensionless integer", name),
            span: None,
        });
    }
    let (n, _) = u32::rounding_from(&q.val, RoundingMode::Floor);
    Ok(n)
}
