use std::collections::HashMap;

use crate::dim::DimVec;

pub struct UnitDef {
    pub name: &'static str,
    pub dim: DimVec,
    pub scale: f64,
    pub offset: f64,
}

pub struct Prefix {
    pub symbol: &'static str,
    pub exponent: i8,
}

pub const PREFIXES: &[Prefix] = &[
    Prefix { symbol: "Y", exponent: 24 },
    Prefix { symbol: "Z", exponent: 21 },
    Prefix { symbol: "E", exponent: 18 },
    Prefix { symbol: "P", exponent: 15 },
    Prefix { symbol: "T", exponent: 12 },
    Prefix { symbol: "G", exponent: 9 },
    Prefix { symbol: "M", exponent: 6 },
    Prefix { symbol: "k", exponent: 3 },
    Prefix { symbol: "h", exponent: 2 },
    Prefix { symbol: "da", exponent: 1 },
    Prefix { symbol: "d", exponent: -1 },
    Prefix { symbol: "c", exponent: -2 },
    Prefix { symbol: "m", exponent: -3 },
    Prefix { symbol: "u", exponent: -6 },
    Prefix { symbol: "n", exponent: -9 },
    Prefix { symbol: "p", exponent: -12 },
    Prefix { symbol: "f", exponent: -15 },
    Prefix { symbol: "a", exponent: -18 },
];

/// Preferred display prefixes in priority order (most common first)
pub const DISPLAY_PREFIXES: &[(&str, i8)] = &[
    ("G", 9),
    ("M", 6),
    ("k", 3),
    ("", 0),
    ("m", -3),
    ("u", -6),
    ("n", -9),
    ("p", -12),
];

pub struct UnitRegistry {
    units: HashMap<&'static str, UnitDef>,
    /// Reverse lookup: DimVec -> best display name
    derived_names: Vec<(DimVec, &'static str)>,
}

impl UnitRegistry {
    pub fn new() -> Self {
        let mut reg = UnitRegistry {
            units: HashMap::new(),
            derived_names: Vec::new(),
        };
        reg.register_defaults();
        reg
    }

    fn register_defaults(&mut self) {
        // SI base units
        self.add("m",   [1, 0, 0, 0, 0, 0, 0], 1.0, 0.0);
        self.add("g",   [0, 1, 0, 0, 0, 0, 0], 1e-3, 0.0); // base SI is kg
        self.add("s",   [0, 0, 1, 0, 0, 0, 0], 1.0, 0.0);
        self.add("A",   [0, 0, 0, 1, 0, 0, 0], 1.0, 0.0);
        self.add("K",   [0, 0, 0, 0, 1, 0, 0], 1.0, 0.0);
        self.add("mol", [0, 0, 0, 0, 0, 1, 0], 1.0, 0.0);
        self.add("cd",  [0, 0, 0, 0, 0, 0, 1], 1.0, 0.0);

        // SI derived units
        self.add_derived("N",  [1, 1, -2, 0, 0, 0, 0], 1.0);  // newton
        self.add_derived("J",  [2, 1, -2, 0, 0, 0, 0], 1.0);  // joule
        self.add_derived("W",  [2, 1, -3, 0, 0, 0, 0], 1.0);  // watt
        self.add_derived("Pa", [-1, 1, -2, 0, 0, 0, 0], 1.0); // pascal
        self.add_derived("V",  [2, 1, -3, -1, 0, 0, 0], 1.0); // volt
        self.add_derived("Ohm",[2, 1, -3, -2, 0, 0, 0], 1.0); // ohm
        self.add_derived("F",  [-2, -1, 4, 2, 0, 0, 0], 1.0); // farad
        self.add_derived("Hz", [0, 0, -1, 0, 0, 0, 0], 1.0);  // hertz
        self.add_derived("C",  [0, 0, 1, 1, 0, 0, 0], 1.0);   // coulomb
        self.add_derived("H",  [2, 1, -2, -2, 0, 0, 0], 1.0); // henry
        self.add_derived("T",  [0, 1, -2, -1, 0, 0, 0], 1.0); // tesla
        self.add_derived("Wb", [2, 1, -2, -1, 0, 0, 0], 1.0); // weber
        self.add_derived("lm", [0, 0, 0, 0, 0, 0, 1], 1.0);   // lumen
        self.add_derived("lx", [-2, 0, 0, 0, 0, 0, 1], 1.0);  // lux

        // Time derived
        self.add("min", [0, 0, 1, 0, 0, 0, 0], 60.0, 0.0);
        self.add("hr",  [0, 0, 1, 0, 0, 0, 0], 3600.0, 0.0);
        self.add("day", [0, 0, 1, 0, 0, 0, 0], 86400.0, 0.0);

        // Imperial / common
        self.add("ft",  [1, 0, 0, 0, 0, 0, 0], 0.3048, 0.0);
        self.add("in",  [1, 0, 0, 0, 0, 0, 0], 0.0254, 0.0);
        self.add("yd",  [1, 0, 0, 0, 0, 0, 0], 0.9144, 0.0);
        self.add("mi",  [1, 0, 0, 0, 0, 0, 0], 1609.344, 0.0);
        self.add("lb",  [0, 1, 0, 0, 0, 0, 0], 0.45359237, 0.0);
        self.add("oz",  [0, 1, 0, 0, 0, 0, 0], 0.028349523125, 0.0);
        self.add("mph", [1, 0, -1, 0, 0, 0, 0], 0.44704, 0.0);
        self.add("L",   [3, 0, 0, 0, 0, 0, 0], 1e-3, 0.0); // liter
        self.add("gal", [3, 0, 0, 0, 0, 0, 0], 3.785411784e-3, 0.0);
        self.add("bar", [-1, 1, -2, 0, 0, 0, 0], 1e5, 0.0);
        self.add("atm", [-1, 1, -2, 0, 0, 0, 0], 101325.0, 0.0);
        self.add("eV",  [2, 1, -2, 0, 0, 0, 0], 1.602176634e-19, 0.0);
        self.add("cal", [2, 1, -2, 0, 0, 0, 0], 4.184, 0.0);

        // Affine temperature units
        self.add("degC", [0, 0, 0, 0, 1, 0, 0], 1.0, 273.15);
        self.add("degF", [0, 0, 0, 0, 1, 0, 0], 5.0 / 9.0, 459.67 * 5.0 / 9.0);
    }

    fn add(&mut self, name: &'static str, dim: [i8; 7], scale: f64, offset: f64) {
        self.units.insert(name, UnitDef {
            name,
            dim: DimVec::new(dim),
            scale,
            offset,
        });
    }

    fn add_derived(&mut self, name: &'static str, dim: [i8; 7], scale: f64) {
        let dv = DimVec::new(dim);
        self.units.insert(name, UnitDef {
            name,
            dim: dv,
            scale,
            offset: 0.0,
        });
        self.derived_names.push((dv, name));
    }

    /// Resolve a unit string like "km", "mV", "N", "degC".
    /// Returns (DimVec, total_scale_to_SI, offset).
    pub fn resolve(&self, unit_str: &str) -> Option<(DimVec, f64, f64)> {
        // Try exact match first
        if let Some(def) = self.units.get(unit_str) {
            return Some((def.dim, def.scale, def.offset));
        }

        // Try prefix + base unit
        for prefix in PREFIXES {
            if let Some(base) = unit_str.strip_prefix(prefix.symbol) {
                if !base.is_empty() {
                    if let Some(def) = self.units.get(base) {
                        let prefix_scale = 10f64.powi(prefix.exponent as i32);
                        return Some((def.dim, def.scale * prefix_scale, def.offset));
                    }
                }
            }
        }

        None
    }

    /// Find the best derived unit name for a dimension vector.
    pub fn find_derived_name(&self, dim: DimVec) -> Option<&'static str> {
        for (d, name) in &self.derived_names {
            if *d == dim {
                return Some(name);
            }
        }
        None
    }

    /// Select the best prefix to display a value in a given unit.
    /// Returns (scaled_value, prefix_symbol).
    pub fn best_prefix(val_in_base: f64) -> (&'static str, f64) {
        let abs = val_in_base.abs();
        if abs == 0.0 {
            return ("", val_in_base);
        }
        for &(sym, exp) in DISPLAY_PREFIXES {
            let scale = 10f64.powi(exp as i32);
            let scaled = abs / scale;
            if scaled >= 1.0 && scaled < 1000.0 {
                return (sym, val_in_base / scale);
            }
        }
        // Fallback: no prefix
        ("", val_in_base)
    }

    pub fn get(&self, name: &str) -> Option<&UnitDef> {
        self.units.get(name)
    }
}

impl Default for UnitRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_base_units() {
        let reg = UnitRegistry::new();
        let (dim, scale, _) = reg.resolve("m").unwrap();
        assert_eq!(dim, DimVec::new([1, 0, 0, 0, 0, 0, 0]));
        assert_eq!(scale, 1.0);
    }

    #[test]
    fn resolve_prefixed() {
        let reg = UnitRegistry::new();
        let (dim, scale, _) = reg.resolve("km").unwrap();
        assert_eq!(dim, DimVec::new([1, 0, 0, 0, 0, 0, 0]));
        assert!((scale - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn resolve_derived() {
        let reg = UnitRegistry::new();
        let (dim, scale, _) = reg.resolve("N").unwrap();
        assert_eq!(dim, DimVec::new([1, 1, -2, 0, 0, 0, 0]));
        assert_eq!(scale, 1.0);
    }

    #[test]
    fn resolve_prefixed_derived() {
        let reg = UnitRegistry::new();
        let (dim, scale, _) = reg.resolve("kN").unwrap();
        assert_eq!(dim, DimVec::new([1, 1, -2, 0, 0, 0, 0]));
        assert!((scale - 1000.0).abs() < 1e-10);
    }

    #[test]
    fn best_prefix_selection() {
        let (prefix, val) = UnitRegistry::best_prefix(0.001);
        assert_eq!(prefix, "m");
        assert!((val - 1.0).abs() < 1e-10);

        let (prefix, val) = UnitRegistry::best_prefix(2_500_000.0);
        assert_eq!(prefix, "M");
        assert!((val - 2.5).abs() < 1e-10);
    }
}
