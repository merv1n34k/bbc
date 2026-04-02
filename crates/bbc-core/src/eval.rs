use malachite::Rational;
use malachite_base::num::conversion::traits::{IsInteger, RoundingFrom};
use malachite_base::rounding_modes::RoundingMode;

use crate::ast::*;
use crate::dim::DimVec;
use crate::env::Env;
use crate::error::Error;
use crate::units::UnitRegistry;
use crate::value::{Quantity, UnitLabel, Value};

pub struct Evaluator {
    pub registry: UnitRegistry,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator {
            registry: UnitRegistry::new(),
        }
    }

    pub fn eval(&self, expr: &Expr, env: &mut Env) -> Result<Value, Error> {
        match expr {
            Expr::Number { value, base } => self.eval_number(value, *base, env.sigfig_mode()),

            Expr::Bool(b) => Ok(Value::Bool(*b)),

            Expr::StringLit(s) => Ok(Value::String(s.clone())),

            Expr::Ident(name) => {
                env.get_var(name)
                    .cloned()
                    .ok_or_else(|| Error::UnknownVariable {
                        name: name.clone(),
                        span: None,
                    })
            }

            Expr::Unary { op, expr } => {
                let val = self.eval(expr, env)?;
                self.eval_unary(*op, val)
            }

            Expr::Binary { op, left, right } => {
                let lval = self.eval(left, env)?;
                let rval = self.eval(right, env)?;
                self.eval_binary(*op, lval, rval)
            }

            Expr::Call { name, args } => {
                let mut eval_args = Vec::with_capacity(args.len());
                for arg in args {
                    eval_args.push(self.eval(arg, env)?);
                }

                // Check modules first
                if let Some(result) = env.call_module_fn(name, &eval_args) {
                    return result;
                }

                // Built-in functions
                self.call_builtin(name, &eval_args)
            }

            Expr::WithUnit { expr, unit } => {
                let val = self.eval(expr, env)?;
                self.apply_unit(val, unit)
            }

            Expr::Convert { expr, target, base } => {
                let val = self.eval(expr, env)?;
                let mut result = if let Some(target) = target {
                    self.convert_unit(val, target, env)?
                } else {
                    val
                };
                if let Some(b) = base {
                    if let Value::Quantity(ref mut q) = result {
                        q.display_base = Some(*b);
                    }
                }
                Ok(result)
            }

            Expr::Assign { name, expr } => {
                let val = self.eval(expr, env)?;
                env.set_var(name.clone(), val.clone())?;
                Ok(val)
            }

            Expr::ConstAssign { name, expr } => {
                if env.get_var(name).is_some() {
                    return Err(Error::TypeError {
                        msg: format!("'{}' is already defined", name),
                        span: None,
                    });
                }
                let val = self.eval(expr, env)?;
                env.set_constant(name.clone(), val.clone());
                Ok(val)
            }
        }
    }

    fn eval_number(&self, value: &str, base: u32, sigfig_mode: bool) -> Result<Value, Error> {
        if base == 10 {
            self.parse_decimal(value, sigfig_mode)
        } else {
            self.parse_based_number(value, base)
        }
    }

    fn parse_decimal(&self, s: &str, sigfig_mode: bool) -> Result<Value, Error> {
        // Handle scientific notation
        if let Some(e_pos) = s.find(|c: char| c == 'e' || c == 'E') {
            let mantissa_str = &s[..e_pos];
            let exp_str = &s[e_pos + 1..];
            let mantissa = self.parse_decimal_simple(mantissa_str)?;
            let exp: i32 = exp_str.parse().map_err(|_| Error::ParseError {
                msg: format!("invalid exponent: {}", exp_str),
                span: None,
            })?;

            let ten = Rational::from(10);
            let scale = rational_pow(&ten, exp);
            let mut q = mantissa.into_quantity().unwrap();
            q.val = q.val * scale;
            // Sigfigs from mantissa digits
            if sigfig_mode {
                q.sigfigs = Some(count_sigfigs(mantissa_str));
            }
            return Ok(Value::Quantity(q));
        }

        let mut val = self.parse_decimal_simple(s)?;
        if sigfig_mode {
            if let Value::Quantity(ref mut q) = val {
                if s.contains('.') {
                    q.sigfigs = Some(count_sigfigs(s));
                }
                // integers are exact (None)
            }
        }
        Ok(val)
    }

    fn parse_decimal_simple(&self, s: &str) -> Result<Value, Error> {
        if let Some(dot_pos) = s.find('.') {
            let int_part = &s[..dot_pos];
            let frac_part = &s[dot_pos + 1..];
            let frac_len = frac_part.len() as u32;

            let int_val: i64 = if int_part.is_empty() {
                0
            } else {
                int_part.parse().map_err(|_| Error::ParseError {
                    msg: format!("invalid number: {}", s),
                    span: None,
                })?
            };

            let frac_val: i64 = frac_part.parse().map_err(|_| Error::ParseError {
                msg: format!("invalid number: {}", s),
                span: None,
            })?;

            let denom = 10i64.pow(frac_len);
            let numerator = int_val * denom + frac_val;
            Ok(Value::from_rational(Rational::from_signeds(
                numerator, denom,
            )))
        } else {
            let n: i64 = s.parse().map_err(|_| Error::ParseError {
                msg: format!("invalid number: {}", s),
                span: None,
            })?;
            Ok(Value::from_int(n))
        }
    }

    fn parse_based_number(&self, digits: &str, base: u32) -> Result<Value, Error> {
        let base_r = Rational::from(base as i64);
        if let Some(dot_pos) = digits.find('.') {
            let int_part = &digits[..dot_pos];
            let frac_part = &digits[dot_pos + 1..];
            let mut int_val = Rational::from(0);
            for ch in int_part.chars() {
                let d = digit_value(ch);
                int_val = int_val * &base_r + Rational::from(d as i64);
            }
            let mut frac_val = Rational::from(0);
            let mut place = Rational::from(1);
            for ch in frac_part.chars() {
                let d = digit_value(ch);
                place = place * &base_r;
                frac_val = frac_val + Rational::from_signeds(d as i64, 1) / &place;
            }
            Ok(Value::from_rational(int_val + frac_val))
        } else {
            let mut result = Rational::from(0);
            for ch in digits.chars() {
                let d = digit_value(ch);
                result = result * base_r.clone() + Rational::from(d as i64);
            }
            Ok(Value::from_rational(result))
        }
    }

    fn eval_unary(&self, op: UnaryOp, val: Value) -> Result<Value, Error> {
        match op {
            UnaryOp::Neg => {
                let q = require_quantity(val, "negation")?;
                Ok(Value::Quantity(Quantity::new(-q.val, q.dim)))
            }
            UnaryOp::Not => match val {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                _ => Err(Error::TypeError {
                    msg: format!("'!' requires bool, got {}", val.type_name()),
                    span: None,
                }),
            },
            UnaryOp::BitNot => {
                let q = require_dimensionless_integer(val, "bitwise NOT")?;
                let (n, _) =
                    malachite_nz::integer::Integer::rounding_from(q.val, RoundingMode::Floor);
                Ok(Value::from_rational(Rational::from(!n)))
            }
        }
    }

    fn eval_binary(&self, op: BinOp, left: Value, right: Value) -> Result<Value, Error> {
        match op {
            BinOp::Add => {
                let l = require_quantity(left, "addition")?;
                let r = require_quantity(right, "addition")?;
                if !l.dim.compatible(r.dim) {
                    return Err(Error::DimensionMismatch {
                        left: l.dim,
                        right: r.dim,
                        span: None,
                    });
                }
                let unit = merge_unit_labels(&l.unit, &r.unit);
                let sigfigs = combine_sigfigs_add(l.sigfigs, r.sigfigs);
                Ok(Value::Quantity(Quantity { val: l.val + r.val, dim: l.dim, unit, display_base: None, sigfigs }))
            }
            BinOp::Sub => {
                let l = require_quantity(left, "subtraction")?;
                let r = require_quantity(right, "subtraction")?;
                if !l.dim.compatible(r.dim) {
                    return Err(Error::DimensionMismatch {
                        left: l.dim,
                        right: r.dim,
                        span: None,
                    });
                }
                let unit = merge_unit_labels(&l.unit, &r.unit);
                let sigfigs = combine_sigfigs_add(l.sigfigs, r.sigfigs);
                Ok(Value::Quantity(Quantity { val: l.val - r.val, dim: l.dim, unit, display_base: None, sigfigs }))
            }
            BinOp::Mul => {
                let l = require_quantity(left, "multiplication")?;
                let r = require_quantity(right, "multiplication")?;
                let sigfigs = combine_sigfigs_mul(l.sigfigs, r.sigfigs);
                let mut q = Quantity::new(l.val * r.val, l.dim.mul(r.dim));
                q.sigfigs = sigfigs;
                Ok(Value::Quantity(q))
            }
            BinOp::Div => {
                let l = require_quantity(left, "division")?;
                let r = require_quantity(right, "division")?;
                if r.val == Rational::from(0) {
                    return Err(Error::DivisionByZero { span: None });
                }
                let sigfigs = combine_sigfigs_mul(l.sigfigs, r.sigfigs);
                let mut q = Quantity::new(l.val / r.val, l.dim.div(r.dim));
                q.sigfigs = sigfigs;
                Ok(Value::Quantity(q))
            }
            BinOp::Mod => {
                let l = require_quantity(left, "modulo")?;
                let r = require_quantity(right, "modulo")?;
                if !l.dim.compatible(r.dim) {
                    return Err(Error::DimensionMismatch {
                        left: l.dim,
                        right: r.dim,
                        span: None,
                    });
                }
                if r.val == Rational::from(0) {
                    return Err(Error::DivisionByZero { span: None });
                }
                // a % b = a - floor(a/b) * b
                let div = &l.val / &r.val;
                let (floored, _) =
                    malachite_nz::integer::Integer::rounding_from(div, RoundingMode::Floor);
                let result = l.val - Rational::from(floored) * r.val;
                Ok(Value::Quantity(Quantity::new(result, l.dim)))
            }
            BinOp::Pow => {
                let l = require_quantity(left, "exponentiation")?;
                let r = require_quantity(right, "exponentiation")?;
                if !r.dim.is_dimensionless() {
                    return Err(Error::TypeError {
                        msg: "exponent must be dimensionless".to_string(),
                        span: None,
                    });
                }
                // For integer exponents, use exact arithmetic
                if r.val.is_integer() {
                    let (exp_int, _) =
                        i32::rounding_from(&r.val, RoundingMode::Floor);
                    let exp_i8 = exp_int as i8;
                    let new_dim = l.dim.pow(exp_i8);
                    let result = rational_pow(&l.val, exp_int);
                    Ok(Value::Quantity(Quantity::new(result, new_dim)))
                } else {
                    // Non-integer exponent: convert to f64
                    let (base_f, _) = f64::rounding_from(&l.val, RoundingMode::Nearest);
                    let (exp_f, _) = f64::rounding_from(&r.val, RoundingMode::Nearest);
                    let result = base_f.powf(exp_f);
                    let r = Rational::try_from(result)
                        .unwrap_or_else(|_| Rational::from(result as i64));
                    // For non-integer powers, dimension must be dimensionless
                    if !l.dim.is_dimensionless() {
                        return Err(Error::TypeError {
                            msg: "non-integer power of dimensioned quantity".to_string(),
                            span: None,
                        });
                    }
                    Ok(Value::Quantity(Quantity::new(r, DimVec::DIMENSIONLESS)))
                }
            }

            // Bitwise ops
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                let l = require_dimensionless_integer(left, "bitwise op")?;
                let r = require_dimensionless_integer(right, "bitwise op")?;
                let (li, _) =
                    malachite_nz::integer::Integer::rounding_from(l.val, RoundingMode::Floor);
                let (ri, _) =
                    malachite_nz::integer::Integer::rounding_from(r.val, RoundingMode::Floor);
                let result = match op {
                    BinOp::BitAnd => li & ri,
                    BinOp::BitOr => li | ri,
                    BinOp::BitXor => li ^ ri,
                    BinOp::Shl => {
                        let (shift, _) = u64::rounding_from(&Rational::from(ri), RoundingMode::Floor);
                        li << shift
                    }
                    BinOp::Shr => {
                        let (shift, _) = u64::rounding_from(&Rational::from(ri), RoundingMode::Floor);
                        li >> shift
                    }
                    _ => unreachable!(),
                };
                Ok(Value::from_rational(Rational::from(result)))
            }

            // Comparison
            BinOp::Eq => Ok(Value::Bool(values_equal(&left, &right))),
            BinOp::Ne => Ok(Value::Bool(!values_equal(&left, &right))),
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let l = require_quantity(left, "comparison")?;
                let r = require_quantity(right, "comparison")?;
                if !l.dim.compatible(r.dim) {
                    return Err(Error::DimensionMismatch {
                        left: l.dim,
                        right: r.dim,
                        span: None,
                    });
                }
                let result = match op {
                    BinOp::Lt => l.val < r.val,
                    BinOp::Le => l.val <= r.val,
                    BinOp::Gt => l.val > r.val,
                    BinOp::Ge => l.val >= r.val,
                    _ => unreachable!(),
                };
                Ok(Value::Bool(result))
            }

            // Logical
            BinOp::And => match (left, right) {
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a && b)),
                (a, b) => Err(Error::TypeError {
                    msg: format!("'&&' requires bool, got {} and {}", a.type_name(), b.type_name()),
                    span: None,
                }),
            },
            BinOp::Or => match (left, right) {
                (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a || b)),
                (a, b) => Err(Error::TypeError {
                    msg: format!("'||' requires bool, got {} and {}", a.type_name(), b.type_name()),
                    span: None,
                }),
            },
        }
    }

    fn apply_unit(&self, val: Value, unit: &UnitExpr) -> Result<Value, Error> {
        let q = require_quantity(val, "unit annotation")?;
        if !q.dim.is_dimensionless() {
            return Err(Error::TypeError {
                msg: "cannot apply unit to already-dimensioned value".to_string(),
                span: None,
            });
        }

        let (dim, scale, offset) = self.resolve_unit_expr(unit)?;
        let scale_r = Rational::try_from(scale).unwrap_or_else(|_| Rational::from(1));
        let offset_r = Rational::try_from(offset).unwrap_or_else(|_| Rational::from(0));
        // For affine units: SI_val = user_val * scale + offset
        // E.g., 25 [degC] -> SI = 25 * 1.0 + 273.15 = 298.15 K
        let si_val = &q.val * &scale_r + &offset_r;
        let label = UnitLabel {
            name: format_unit_expr(unit),
            scale: scale_r,
            offset: offset_r,
        };
        Ok(Value::Quantity(Quantity::with_unit(si_val, dim, label)))
    }

    fn convert_unit(
        &self,
        val: Value,
        target: &UnitExpr,
        _env: &Env,
    ) -> Result<Value, Error> {
        let q = require_quantity(val, "unit conversion")?;
        let (target_dim, target_scale, target_offset) = self.resolve_unit_expr(target)?;

        if !q.dim.compatible(target_dim) {
            return Err(Error::DimensionMismatch {
                left: q.dim,
                right: target_dim,
                span: None,
            });
        }

        let target_scale_r =
            Rational::try_from(target_scale).unwrap_or_else(|_| Rational::from(1));
        let target_offset_r =
            Rational::try_from(target_offset).unwrap_or_else(|_| Rational::from(0));
        let target_name = format_unit_expr(target);
        let label = UnitLabel {
            name: target_name,
            scale: target_scale_r,
            offset: target_offset_r,
        };
        // Keep val in SI, attach unit label for display
        Ok(Value::Quantity(Quantity::with_unit(q.val, q.dim, label)))
    }

    /// Resolve a parsed UnitExpr to (DimVec, combined_scale_to_SI, offset).
    /// Offset is only non-zero for single affine units (e.g., degC, degF).
    fn resolve_unit_expr(&self, unit: &UnitExpr) -> Result<(DimVec, f64, f64), Error> {
        let mut combined_dim = DimVec::DIMENSIONLESS;
        let mut combined_scale = 1.0;
        let mut combined_offset = 0.0;

        for part in &unit.parts {
            let (dim, scale, offset) =
                self.registry.resolve(&part.name).ok_or_else(|| Error::UnknownUnit {
                    name: part.name.clone(),
                    span: None,
                })?;

            // Offset only valid for single unit with exp=1
            if offset != 0.0 && unit.parts.len() == 1 && part.exp == 1 {
                combined_offset = offset;
            }

            if part.exp == 1 {
                combined_dim = combined_dim.mul(dim);
                combined_scale *= scale;
            } else if part.exp == -1 {
                combined_dim = combined_dim.div(dim);
                combined_scale /= scale;
            } else {
                combined_dim = combined_dim.mul(dim.pow(part.exp));
                combined_scale *= scale.powi(part.exp as i32);
            }
        }

        Ok((combined_dim, combined_scale, combined_offset))
    }

    fn call_builtin(&self, name: &str, args: &[Value]) -> Result<Value, Error> {
        match name {
            // Trig functions
            "sin" => unary_f64_fn(args, "sin", f64::sin),
            "cos" => unary_f64_fn(args, "cos", f64::cos),
            "tan" => unary_f64_fn(args, "tan", f64::tan),
            "asin" => unary_f64_fn(args, "asin", f64::asin),
            "acos" => unary_f64_fn(args, "acos", f64::acos),
            "atan" => unary_f64_fn(args, "atan", f64::atan),
            "atan2" => {
                require_n_args(args, 2, "atan2")?;
                let y = require_dimensionless_f64(&args[0], "atan2")?;
                let x = require_dimensionless_f64(&args[1], "atan2")?;
                Ok(Value::from_rational(
                    Rational::try_from(y.atan2(x)).unwrap(),
                ))
            }

            // Hyperbolic
            "sinh" => unary_f64_fn(args, "sinh", f64::sinh),
            "cosh" => unary_f64_fn(args, "cosh", f64::cosh),
            "tanh" => unary_f64_fn(args, "tanh", f64::tanh),

            // Exponential / logarithmic
            "exp" => unary_f64_fn(args, "exp", f64::exp),
            "ln" => unary_f64_fn(args, "ln", f64::ln),
            "log2" => unary_f64_fn(args, "log2", f64::log2),
            "log10" => unary_f64_fn(args, "log10", f64::log10),
            "log" => {
                if args.len() == 1 {
                    unary_f64_fn(args, "log", f64::ln)
                } else if args.len() == 2 {
                    let val = require_dimensionless_f64(&args[0], "log")?;
                    let base = require_dimensionless_f64(&args[1], "log")?;
                    Ok(Value::from_rational(
                        Rational::try_from(val.log(base)).unwrap(),
                    ))
                } else {
                    Err(Error::InvalidArguments {
                        msg: "log takes 1 or 2 arguments".to_string(),
                        span: None,
                    })
                }
            }

            // Power / root
            "sqrt" => {
                require_n_args(args, 1, "sqrt")?;
                let q = require_quantity_ref(&args[0], "sqrt")?;
                let new_dim = q.dim.root(2).ok_or_else(|| Error::InvalidDimensionRoot {
                    dim: q.dim,
                    n: 2,
                    span: None,
                })?;
                let (f, _) = f64::rounding_from(&q.val, RoundingMode::Nearest);
                let result = f.sqrt();
                Ok(Value::Quantity(Quantity::new(
                    Rational::try_from(result).unwrap(),
                    new_dim,
                )))
            }
            "cbrt" => {
                require_n_args(args, 1, "cbrt")?;
                let q = require_quantity_ref(&args[0], "cbrt")?;
                let new_dim = q.dim.root(3).ok_or_else(|| Error::InvalidDimensionRoot {
                    dim: q.dim,
                    n: 3,
                    span: None,
                })?;
                let (f, _) = f64::rounding_from(&q.val, RoundingMode::Nearest);
                let result = f.cbrt();
                Ok(Value::Quantity(Quantity::new(
                    Rational::try_from(result).unwrap(),
                    new_dim,
                )))
            }

            // Rounding
            "abs" => {
                require_n_args(args, 1, "abs")?;
                let q = require_quantity(args[0].clone(), "abs")?;
                let val = if q.val < Rational::from(0) {
                    -q.val
                } else {
                    q.val
                };
                Ok(Value::Quantity(Quantity::new(val, q.dim)))
            }
            "floor" => {
                require_n_args(args, 1, "floor")?;
                let q = require_quantity(args[0].clone(), "floor")?;
                let (n, _) =
                    malachite_nz::integer::Integer::rounding_from(q.val, RoundingMode::Floor);
                Ok(Value::Quantity(Quantity::new(Rational::from(n), q.dim)))
            }
            "ceil" => {
                require_n_args(args, 1, "ceil")?;
                let q = require_quantity(args[0].clone(), "ceil")?;
                let (n, _) =
                    malachite_nz::integer::Integer::rounding_from(q.val, RoundingMode::Ceiling);
                Ok(Value::Quantity(Quantity::new(Rational::from(n), q.dim)))
            }
            "round" => {
                require_n_args(args, 1, "round")?;
                let q = require_quantity(args[0].clone(), "round")?;
                let (n, _) =
                    malachite_nz::integer::Integer::rounding_from(q.val, RoundingMode::Nearest);
                Ok(Value::Quantity(Quantity::new(Rational::from(n), q.dim)))
            }

            // Min/max
            "min" => {
                require_n_args(args, 2, "min")?;
                let a = require_quantity(args[0].clone(), "min")?;
                let b = require_quantity(args[1].clone(), "min")?;
                if !a.dim.compatible(b.dim) {
                    return Err(Error::DimensionMismatch {
                        left: a.dim,
                        right: b.dim,
                        span: None,
                    });
                }
                Ok(Value::Quantity(if a.val <= b.val {
                    a
                } else {
                    b
                }))
            }
            "max" => {
                require_n_args(args, 2, "max")?;
                let a = require_quantity(args[0].clone(), "max")?;
                let b = require_quantity(args[1].clone(), "max")?;
                if !a.dim.compatible(b.dim) {
                    return Err(Error::DimensionMismatch {
                        left: a.dim,
                        right: b.dim,
                        span: None,
                    });
                }
                Ok(Value::Quantity(if a.val >= b.val {
                    a
                } else {
                    b
                }))
            }

            _ => Err(Error::UnknownFunction {
                name: name.to_string(),
                span: None,
            }),
        }
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

// Helper: compute rational^int
fn rational_pow(base: &Rational, exp: i32) -> Rational {
    if exp == 0 {
        return Rational::from(1);
    }
    if exp < 0 {
        let pos = rational_pow(base, -exp);
        return Rational::from(1) / pos;
    }
    let mut result = Rational::from(1);
    let mut b = base.clone();
    let mut e = exp as u32;
    while e > 0 {
        if e & 1 == 1 {
            result = result * &b;
        }
        b = b.clone() * &b;
        e >>= 1;
    }
    result
}

fn require_quantity(val: Value, context: &str) -> Result<Quantity, Error> {
    val.into_quantity().ok_or_else(|| Error::TypeError {
        msg: format!("{} requires a number", context),
        span: None,
    })
}

fn require_quantity_ref<'a>(val: &'a Value, context: &str) -> Result<&'a Quantity, Error> {
    val.as_quantity().ok_or_else(|| Error::TypeError {
        msg: format!("{} requires a number", context),
        span: None,
    })
}

fn require_dimensionless_integer(val: Value, context: &str) -> Result<Quantity, Error> {
    let q = require_quantity(val, context)?;
    if !q.dim.is_dimensionless() {
        return Err(Error::TypeError {
            msg: format!("{} requires dimensionless value", context),
            span: None,
        });
    }
    if !q.val.is_integer() {
        return Err(Error::TypeError {
            msg: format!("{} requires integer value", context),
            span: None,
        });
    }
    Ok(q)
}

fn require_dimensionless_f64(val: &Value, context: &str) -> Result<f64, Error> {
    let q = require_quantity_ref(val, context)?;
    if !q.dim.is_dimensionless() {
        return Err(Error::TypeError {
            msg: format!("{} requires dimensionless value", context),
            span: None,
        });
    }
    let (f, _) = f64::rounding_from(&q.val, RoundingMode::Nearest);
    Ok(f)
}

fn require_n_args(args: &[Value], n: usize, name: &str) -> Result<(), Error> {
    if args.len() != n {
        return Err(Error::InvalidArguments {
            msg: format!("{} requires {} argument(s), got {}", name, n, args.len()),
            span: None,
        });
    }
    Ok(())
}

fn unary_f64_fn(
    args: &[Value],
    name: &str,
    f: fn(f64) -> f64,
) -> Result<Value, Error> {
    require_n_args(args, 1, name)?;
    let x = require_dimensionless_f64(&args[0], name)?;
    let result = f(x);
    Ok(Value::from_rational(
        Rational::try_from(result).unwrap_or_else(|_| Rational::from(result as i64)),
    ))
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Quantity(qa), Value::Quantity(qb)) => qa.dim == qb.dim && qa.val == qb.val,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        _ => false,
    }
}

fn count_sigfigs(s: &str) -> u32 {
    let s = s.trim_start_matches('-');
    if s.contains('.') {
        // All digits are significant (leading zeros before decimal don't count,
        // but trailing zeros after decimal do)
        let without_dot = s.replace('.', "");
        let trimmed = without_dot.trim_start_matches('0');
        if trimmed.is_empty() {
            // "0.0" -> 1 sigfig
            1
        } else {
            trimmed.len() as u32
        }
    } else {
        // Integer in scientific notation mantissa: all digits significant
        let trimmed = s.trim_start_matches('0');
        if trimmed.is_empty() { 1 } else { trimmed.len() as u32 }
    }
}

/// Combine sigfigs for mul/div: result = min(left, right), ignoring exact values (None).
fn combine_sigfigs_mul(a: Option<u32>, b: Option<u32>) -> Option<u32> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

/// Combine sigfigs for add/sub: use the fewer sigfigs (simplified rule).
fn combine_sigfigs_add(a: Option<u32>, b: Option<u32>) -> Option<u32> {
    combine_sigfigs_mul(a, b)
}

fn digit_value(ch: char) -> u32 {
    if ch.is_ascii_digit() {
        ch as u32 - '0' as u32
    } else {
        ch as u32 - 'A' as u32 + 10
    }
}

fn merge_unit_labels(a: &Option<UnitLabel>, b: &Option<UnitLabel>) -> Option<UnitLabel> {
    match (a, b) {
        (Some(la), Some(lb)) if la.name == lb.name => Some(la.clone()),
        _ => None,
    }
}

fn format_unit_expr(unit: &UnitExpr) -> String {
    unit.parts
        .iter()
        .map(|p| {
            if p.exp == 1 {
                p.name.clone()
            } else {
                format!("{}^{}", p.name, p.exp)
            }
        })
        .collect::<Vec<_>>()
        .join("*")
}
