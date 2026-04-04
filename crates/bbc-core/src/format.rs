use malachite::Rational;
use malachite_base::num::conversion::traits::{IsInteger, RoundingFrom};
use malachite_base::rounding_modes::RoundingMode;

use crate::env::{View, ViewSet};
use crate::units::UnitRegistry;
use crate::value::{Quantity, Value};

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

    let negative = *val < 0;
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
        remainder *= ten.clone();
        let (digit, _) = malachite_nz::integer::Integer::rounding_from(remainder.clone(), RoundingMode::Floor);
        let d: u8 = u8::rounding_from(&Rational::from(digit.clone()), RoundingMode::Floor).0;
        digits.push((b'0' + d) as char);
        remainder -= Rational::from(digit);
        if remainder == 0 {
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
    let negative = *val < 0;
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
        remainder *= base_r.clone();
        let (digit, _) = malachite_nz::integer::Integer::rounding_from(remainder.clone(), RoundingMode::Floor);
        let d: u8 = u8::rounding_from(&Rational::from(digit.clone()), RoundingMode::Floor).0;
        digits.push(digit_char(d));
        remainder -= Rational::from(digit);
        if remainder == 0 {
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

    if *val == 0 {
        return "0".to_string();
    }

    let mut n = val.clone();
    let negative = n < 0;
    if negative {
        n = -n;
    }

    let base_i = Integer::from(base);
    let mut digits = Vec::new();
    while n > 0 {
        let remainder = &n % &base_i;
        let (d, _) = u8::rounding_from(&Rational::from(remainder), RoundingMode::Floor);
        digits.push(digit_char(d));
        n /= &base_i;
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

pub fn format_scientific(val: f64, scale: u32) -> String {
    if val == 0.0 {
        return "0".to_string();
    }
    let abs = val.abs();
    let exp = abs.log10().floor() as i32;
    let mantissa = val / 10f64.powi(exp);
    let prec = (scale as usize).clamp(1, 15);
    let m_str = format!("{:.prec$}", mantissa, prec = prec);
    let m_str = m_str.trim_end_matches('0');
    let m_str = m_str.trim_end_matches('.');
    if exp == 0 {
        m_str.to_string()
    } else {
        format!("{}e{}", m_str, exp)
    }
}

fn needs_scientific(val: f64) -> bool {
    if val == 0.0 {
        return false;
    }
    let abs = val.abs();
    !(0.01..999.0).contains(&abs)
}

pub fn format_quantity(q: &Quantity, obase: u32, scale: u32, registry: &UnitRegistry, views: &ViewSet) -> String {
    let effective_base = q.display_base.unwrap_or(obase);
    let use_strict = views.has(View::Strict);
    let use_adjust = views.has(View::Adjust);
    let use_scientific = views.has(View::Scientific);

    // Strict view: force raw SI base units
    if use_strict {
        let num_str = format_with_sigfigs(&q.val, 10, scale, q.sigfigs);
        if q.dim.is_dimensionless() {
            return maybe_scientific_str(&q.val, &num_str, scale, use_scientific);
        }
        let formatted = format!("{} [{}]", num_str, q.dim);
        if use_scientific {
            return apply_scientific_to_formatted(&q.val, &formatted, scale);
        }
        return formatted;
    }

    // If quantity has an explicit unit label, use it
    if let Some(ref label) = q.unit {
        // For affine units (degC, degF): no auto-prefix, display as-is
        if label.offset != 0 {
            let display_val = (&q.val - &label.offset) / &label.scale;
            let num_str = format_with_sigfigs(&display_val, effective_base, scale, q.sigfigs);
            let formatted = format!("{} [{}]", num_str, label.name);
            if use_scientific {
                return apply_scientific_to_formatted(&display_val, &formatted, scale);
            }
            return formatted;
        }

        // Auto-prefix: for non-pinned units when adjust is on
        if use_adjust && !label.pinned
            && let Some(result) = auto_prefix_unit(&q.val, &label.name, scale, registry) {
            if use_scientific {
                let display_val = &q.val / &label.scale;
                let (approx, _) = f64::rounding_from(&display_val, RoundingMode::Nearest);
                if needs_scientific(approx) {
                    let sci_num = format_scientific(approx, scale);
                    return format!("{} [{}]", sci_num, label.name);
                }
            }
            return result;
        }

        // Fallback: display in user's original unit
        let display_val = (&q.val - &label.offset) / &label.scale;
        let num_str = format_with_sigfigs(&display_val, effective_base, scale, q.sigfigs);
        let formatted = format!("{} [{}]", num_str, label.name);
        if use_scientific {
            return apply_scientific_to_formatted(&display_val, &formatted, scale);
        }
        return formatted;
    }

    let num_str = format_with_sigfigs(&q.val, effective_base, scale, q.sigfigs);

    if q.dim.is_dimensionless() {
        return maybe_scientific_str(&q.val, &num_str, scale, use_scientific);
    }

    // Try to find a derived unit name (only when adjust is on)
    if use_adjust
        && let Some(name) = registry.find_derived_name(q.dim)
    {
        let (approx, _) = f64::rounding_from(&q.val, RoundingMode::Nearest);
        let (prefix, display_val) = UnitRegistry::best_prefix(approx);
        if !prefix.is_empty() {
            let formatted = format!("{} [{}{}]", format_f64_trimmed(display_val, scale), prefix, name);
            if use_scientific && needs_scientific(display_val) {
                let sci_num = format_scientific(display_val, scale);
                return format!("{} [{}{}]", sci_num, prefix, name);
            }
            return formatted;
        }
        let formatted = format!("{} [{}]", num_str, name);
        if use_scientific {
            return apply_scientific_to_formatted(&q.val, &formatted, scale);
        }
        return formatted;
    }

    // Fall back to raw dimension display
    let formatted = format!("{} [{}]", num_str, q.dim);
    if use_scientific {
        return apply_scientific_to_formatted(&q.val, &formatted, scale);
    }
    formatted
}

fn maybe_scientific_str(val: &Rational, num_str: &str, scale: u32, use_scientific: bool) -> String {
    if use_scientific {
        let (approx, _) = f64::rounding_from(val, RoundingMode::Nearest);
        if needs_scientific(approx) {
            return format_scientific(approx, scale);
        }
    }
    num_str.to_string()
}

fn apply_scientific_to_formatted(val: &Rational, formatted: &str, scale: u32) -> String {
    let (approx, _) = f64::rounding_from(val, RoundingMode::Nearest);
    if needs_scientific(approx) {
        let sci_num = format_scientific(approx, scale);
        // Replace the numeric part before the first ' ['
        if let Some(bracket_pos) = formatted.find(" [") {
            return format!("{}{}", sci_num, &formatted[bracket_pos..]);
        }
    }
    formatted.to_string()
}

fn auto_prefix_unit(
    si_val: &Rational,
    label_name: &str,
    scale: u32,
    registry: &UnitRegistry,
) -> Option<String> {
    let parts = parse_label_parts(label_name);
    if parts.is_empty() {
        return None;
    }

    let prefix_idx = parts.iter().position(|(name, _exp)| {
        registry.base_unit_name(name).is_some()
    })?;

    let (part_name, _part_exp) = &parts[prefix_idx];
    let (base_name, _existing_prefix_exp) = registry.base_unit_name(part_name)?;
    let (_base_dim, base_scale, _) = registry.resolve(base_name)?;

    let mut other_scale = 1.0;
    for (i, (name, exp)) in parts.iter().enumerate() {
        if i == prefix_idx {
            continue;
        }
        let (_, s, _) = registry.resolve(name)?;
        other_scale *= s.powi(*exp);
    }

    let base_scale_r = Rational::try_from(base_scale * other_scale)
        .unwrap_or_else(|_| Rational::from(1));
    let val_in_base = si_val / &base_scale_r;
    let (approx, _) = f64::rounding_from(&val_in_base, RoundingMode::Nearest);
    let (prefix, display_val) = UnitRegistry::best_prefix(approx);

    let new_parts: Vec<String> = parts
        .iter()
        .enumerate()
        .map(|(i, (name, exp))| {
            let unit_name = if i == prefix_idx {
                format!("{}{}", prefix, base_name)
            } else {
                name.to_string()
            };
            if *exp == 1 {
                unit_name
            } else {
                format!("{}^{}", unit_name, exp)
            }
        })
        .collect();
    let unit_str = new_parts.join("*");

    Some(format!("{} [{}]", format_f64_trimmed(display_val, scale), unit_str))
}

fn parse_label_parts(label: &str) -> Vec<(&str, i32)> {
    label
        .split('*')
        .map(|part| {
            if let Some((name, exp_str)) = part.split_once('^') {
                let exp: i32 = exp_str.parse().unwrap_or(1);
                (name, exp)
            } else {
                (part, 1)
            }
        })
        .collect()
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
        format!("{:.prec$}", rounded, prec = decimal_places)
    }
}

fn format_f64_trimmed(val: f64, scale: u32) -> String {
    let s = format!("{:.prec$}", val, prec = scale as usize);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

pub fn format_value(val: &Value, obase: u32, scale: u32, registry: &UnitRegistry, views: &ViewSet) -> String {
    match val {
        Value::Quantity(q) => format_quantity(q, obase, scale, registry, views),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
    }
}

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

    #[test]
    fn scientific_notation() {
        let s = format_scientific(1500.0, 6);
        assert_eq!(s, "1.5e3");
        let s = format_scientific(0.001, 6);
        assert_eq!(s, "1e-3");
        let s = format_scientific(6.626e-34, 6);
        assert!(s.starts_with("6.626e-34"));
    }
}
