use malachite::Rational;
use malachite_base::num::conversion::traits::{IsInteger, RoundingFrom};
use malachite_base::rounding_modes::RoundingMode;

use crate::units::UnitRegistry;
use crate::value::{Quantity, Value};

/// Format a rational number in the given output base with the given decimal precision.
pub fn format_rational(val: &Rational, obase: u32, scale: u32) -> String {
    if obase == 10 {
        format_rational_base10(val, scale)
    } else {
        format_rational_arbitrary_base(val, obase, scale)
    }
}

fn format_rational_base10(val: &Rational, scale: u32) -> String {
    if val.is_integer() {
        let (n, _) = malachite_nz::integer::Integer::rounding_from(val.clone(), RoundingMode::Floor);
        return n.to_string();
    }

    let negative = *val < Rational::from(0);
    let abs_val = if negative {
        -val.clone()
    } else {
        val.clone()
    };

    let (int_part, _) = malachite_nz::integer::Integer::rounding_from(abs_val.clone(), RoundingMode::Floor);
    let frac = abs_val - Rational::from(int_part.clone());

    let mut digits = String::new();
    let mut remainder = frac;
    let ten = Rational::from(10);
    for _ in 0..scale {
        remainder = remainder * ten.clone();
        let (digit, _) = malachite_nz::integer::Integer::rounding_from(remainder.clone(), RoundingMode::Floor);
        let d: u8 = u8::rounding_from(&Rational::from(digit.clone()), RoundingMode::Floor).0;
        digits.push((b'0' + d) as char);
        remainder = remainder - Rational::from(digit);
        if remainder == Rational::from(0) {
            break;
        }
    }

    let trimmed = digits.trim_end_matches('0');
    let prefix = if negative { "-" } else { "" };
    if trimmed.is_empty() {
        format!("{}{}", prefix, int_part)
    } else {
        format!("{}{}.{}", prefix, int_part, trimmed)
    }
}

fn format_rational_arbitrary_base(val: &Rational, base: u32, scale: u32) -> String {
    let negative = *val < Rational::from(0);
    let abs_val = if negative {
        -val.clone()
    } else {
        val.clone()
    };

    let (int_part, _) = malachite_nz::integer::Integer::rounding_from(abs_val.clone(), RoundingMode::Floor);
    let frac = abs_val - Rational::from(int_part.clone());

    let int_str = format_integer_in_base(&int_part, base);

    let mut digits = String::new();
    let mut remainder = frac;
    let base_r = Rational::from(base as i64);
    for _ in 0..scale {
        remainder = remainder * base_r.clone();
        let (digit, _) = malachite_nz::integer::Integer::rounding_from(remainder.clone(), RoundingMode::Floor);
        let d: u8 = u8::rounding_from(&Rational::from(digit.clone()), RoundingMode::Floor).0;
        digits.push(digit_char(d));
        remainder = remainder - Rational::from(digit);
        if remainder == Rational::from(0) {
            break;
        }
    }

    let trimmed = digits.trim_end_matches('0');
    let prefix = if negative { "-" } else { "" };
    let base_prefix = format!("{}x", base);
    if trimmed.is_empty() {
        format!("{}{}{}", prefix, base_prefix, int_str)
    } else {
        format!("{}{}{}.{}", prefix, base_prefix, int_str, trimmed)
    }
}

fn format_integer_in_base(val: &malachite_nz::integer::Integer, base: u32) -> String {
    use malachite_nz::integer::Integer;

    if *val == Integer::from(0) {
        return "0".to_string();
    }

    let mut n = val.clone();
    let negative = n < Integer::from(0);
    if negative {
        n = -n;
    }

    let base_i = Integer::from(base);
    let mut digits = Vec::new();
    while n > Integer::from(0) {
        let remainder = &n % &base_i;
        let (d, _) = u8::rounding_from(&Rational::from(remainder), RoundingMode::Floor);
        digits.push(digit_char(d));
        n = n / &base_i;
    }

    digits.reverse();
    let s: String = digits.into_iter().collect();
    if negative {
        format!("-{}", s)
    } else {
        s
    }
}

fn digit_char(d: u8) -> char {
    if d < 10 {
        (b'0' + d) as char
    } else {
        (b'A' + d - 10) as char
    }
}

/// Format a quantity for display, using the unit registry to find derived names
/// and best prefixes.
pub fn format_quantity(q: &Quantity, obase: u32, scale: u32, registry: &UnitRegistry) -> String {
    let effective_base = q.display_base.unwrap_or(obase);

    // If quantity has an explicit unit label, use it
    if let Some(ref label) = q.unit {
        // For affine units: display_val = (SI_val - offset) / scale
        let display_val = (&q.val - &label.offset) / &label.scale;
        let num_str = format_with_sigfigs(&display_val, effective_base, scale, q.sigfigs);
        return format!("{} [{}]", num_str, label.name);
    }

    let num_str = format_with_sigfigs(&q.val, effective_base, scale, q.sigfigs);

    if q.dim.is_dimensionless() {
        return num_str;
    }

    // Try to find a derived unit name
    if let Some(name) = registry.find_derived_name(q.dim) {
        let (approx, _) = f64::rounding_from(&q.val, RoundingMode::Nearest);
        let (prefix, display_val) = UnitRegistry::best_prefix(approx);
        if !prefix.is_empty() {
            return format!("{} [{}{}]", format_f64_trimmed(display_val, scale), prefix, name);
        }
        return format!("{} [{}]", num_str, name);
    }

    // Fall back to raw dimension display
    format!("{} [{}]", num_str, q.dim)
}

fn format_with_sigfigs(val: &Rational, obase: u32, scale: u32, sigfigs: Option<u32>) -> String {
    match sigfigs {
        Some(sf) if sf > 0 => {
            let (f, _) = f64::rounding_from(val, RoundingMode::Nearest);
            format_f64_sigfigs(f, sf)
        }
        _ => format_rational(val, obase, scale),
    }
}

fn format_f64_sigfigs(val: f64, sigfigs: u32) -> String {
    if val == 0.0 {
        return "0".to_string();
    }
    let magnitude = val.abs().log10().floor() as i32;
    let decimal_places = (sigfigs as i32 - 1 - magnitude).max(0) as usize;
    let rounded = {
        let factor = 10f64.powi(sigfigs as i32 - 1 - magnitude);
        (val * factor).round() / factor
    };
    if decimal_places == 0 {
        format!("{}", rounded as i64)
    } else {
        let s = format!("{:.prec$}", rounded, prec = decimal_places);
        s
    }
}

fn format_f64_trimmed(val: f64, scale: u32) -> String {
    let s = format!("{:.prec$}", val, prec = scale as usize);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

/// Format a value for display
pub fn format_value(val: &Value, obase: u32, scale: u32, registry: &UnitRegistry) -> String {
    match val {
        Value::Quantity(q) => format_quantity(q, obase, scale, registry),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
    }
}

pub fn format_value_strict(val: &Value, scale: u32) -> String {
    match val {
        Value::Quantity(q) => {
            let num_str = format_with_sigfigs(&q.val, 10, scale, q.sigfigs);
            if q.dim.is_dimensionless() {
                num_str
            } else {
                format!("{} [{}]", num_str, q.dim)
            }
        }
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
    }
}

/// Format a quantity with an explicit target unit for `->` conversion display.
/// Divides the SI value by target_scale using exact rational arithmetic.
pub fn format_quantity_in_unit(
    q: &Quantity,
    target_unit: &str,
    target_scale: &Rational,
    obase: u32,
    scale: u32,
) -> String {
    let display_val = &q.val / target_scale;
    let num_str = format_rational(&display_val, obase, scale);
    format!("{} [{}]", num_str, target_unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_integer() {
        let val = Rational::from(42);
        assert_eq!(format_rational(&val, 10, 20), "42");
    }

    #[test]
    fn format_fraction() {
        let val = Rational::from_signeds(1, 3);
        let s = format_rational(&val, 10, 6);
        assert_eq!(s, "0.333333");
    }

    #[test]
    fn format_hex() {
        let val = Rational::from(255);
        assert_eq!(format_rational(&val, 16, 20), "16xFF");
    }

    #[test]
    fn format_binary() {
        let val = Rational::from(10);
        assert_eq!(format_rational(&val, 2, 20), "2x1010");
    }

    #[test]
    fn format_negative() {
        let val = Rational::from(-42);
        assert_eq!(format_rational(&val, 10, 20), "-42");
    }
}
