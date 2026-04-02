use std::collections::HashMap;

use serde::Deserialize;

use crate::dim::DimVec;

include!(concat!(env!("OUT_DIR"), "/unit_sets.rs"));

/// Names of unit sets always loaded (SI derived + common non-SI).
const DEFAULT_SETS: &[&str] = &["derived", "common"];

#[derive(Debug, Clone)]
pub struct UnitDef {
    pub name: String,
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

// -- TOML deserialization types --

#[derive(Deserialize)]
struct TomlUnitFile {
    #[serde(default)]
    units: HashMap<String, TomlUnitDef>,
    #[serde(default)]
    derived: HashMap<String, Vec<i8>>,
}

#[derive(Deserialize)]
struct TomlUnitDef {
    scale: f64,
    dim: Vec<i8>,
    #[serde(default)]
    offset: f64,
}

pub struct UnitRegistry {
    units: HashMap<String, Vec<UnitDef>>,
    /// Reverse lookup: DimVec -> best display name
    derived_names: Vec<(DimVec, String)>,
}

impl UnitRegistry {
    pub fn new() -> Self {
        let mut reg = UnitRegistry {
            units: HashMap::new(),
            derived_names: Vec::new(),
        };
        reg.register_si_base();
        for name in DEFAULT_SETS {
            reg.load_unit_set(name);
        }
        reg
    }

    fn register_si_base(&mut self) {
        self.add("m",   [1, 0, 0, 0, 0, 0, 0], 1.0, 0.0);
        self.add("g",   [0, 1, 0, 0, 0, 0, 0], 1e-3, 0.0);
        self.add("s",   [0, 0, 1, 0, 0, 0, 0], 1.0, 0.0);
        self.add("A",   [0, 0, 0, 1, 0, 0, 0], 1.0, 0.0);
        self.add("K",   [0, 0, 0, 0, 1, 0, 0], 1.0, 0.0);
        self.add("mol", [0, 0, 0, 0, 0, 1, 0], 1.0, 0.0);
        self.add("cd",  [0, 0, 0, 0, 0, 0, 1], 1.0, 0.0);
    }

    fn add(&mut self, name: &str, dim: [i8; 7], scale: f64, offset: f64) {
        let def = UnitDef {
            name: name.to_string(),
            dim: DimVec::new(dim),
            scale,
            offset,
        };
        self.units.entry(name.to_string()).or_default().push(def);
    }

    /// Load units from a TOML string.
    pub fn load_toml(&mut self, content: &str) {
        let file: TomlUnitFile = toml::from_str(content)
            .expect("failed to parse unit TOML");

        for (name, def) in &file.units {
            let dim_arr = to_dim_array(&def.dim);
            let dv = DimVec::new(dim_arr);
            let unit_def = UnitDef {
                name: name.clone(),
                dim: dv,
                scale: def.scale,
                offset: def.offset,
            };

            let entries = self.units.entry(name.clone()).or_default();
            if let Some(existing) = entries.iter_mut().find(|e| e.dim == dv) {
                eprintln!("warning: duplicate unit '{}' ({}) redefined, using last definition", name, dv);
                *existing = unit_def;
            } else {
                entries.push(unit_def);
            }
        }

        for (name, dim_arr) in &file.derived {
            let dv = DimVec::new(to_dim_array(dim_arr));
            if !self.derived_names.iter().any(|(d, _)| *d == dv) {
                self.derived_names.push((dv, name.clone()));
            }
        }
    }

    /// Load a named unit set from embedded TOML data.
    pub fn load_unit_set(&mut self, name: &str) {
        for &(set_name, content) in UNIT_SETS {
            if set_name == name {
                self.load_toml(content);
                return;
            }
        }
        eprintln!("warning: unknown unit set '{}', available: {:?}",
            name, Self::available_unit_sets());
    }

    /// Returns all available unit set names (derived from data/*.toml filenames).
    pub fn available_unit_sets() -> Vec<&'static str> {
        UNIT_SETS.iter().map(|&(name, _)| name).collect()
    }

    /// Resolve a unit string like "km", "mV", "N", "degC".
    /// Returns (DimVec, total_scale_to_SI, offset).
    /// For units with multiple definitions (dimensional overload), returns the first.
    pub fn resolve(&self, unit_str: &str) -> Option<(DimVec, f64, f64)> {
        if let Some(defs) = self.units.get(unit_str) {
            if let Some(def) = defs.first() {
                return Some((def.dim, def.scale, def.offset));
            }
        }

        for prefix in PREFIXES {
            if let Some(base) = unit_str.strip_prefix(prefix.symbol) {
                if !base.is_empty() {
                    if let Some(defs) = self.units.get(base) {
                        if let Some(def) = defs.first() {
                            let prefix_scale = 10f64.powi(prefix.exponent as i32);
                            return Some((def.dim, def.scale * prefix_scale, def.offset));
                        }
                    }
                }
            }
        }

        None
    }

    /// Resolve with dimensional overloads. Returns all matching definitions.
    pub fn resolve_all(&self, unit_str: &str) -> Vec<(DimVec, f64, f64)> {
        if let Some(defs) = self.units.get(unit_str) {
            return defs.iter().map(|d| (d.dim, d.scale, d.offset)).collect();
        }

        for prefix in PREFIXES {
            if let Some(base) = unit_str.strip_prefix(prefix.symbol) {
                if !base.is_empty() {
                    if let Some(defs) = self.units.get(base) {
                        let prefix_scale = 10f64.powi(prefix.exponent as i32);
                        return defs.iter()
                            .map(|d| (d.dim, d.scale * prefix_scale, d.offset))
                            .collect();
                    }
                }
            }
        }

        Vec::new()
    }

    /// Find the best derived unit name for a dimension vector.
    pub fn find_derived_name(&self, dim: DimVec) -> Option<&str> {
        for (d, name) in &self.derived_names {
            if *d == dim {
                return Some(name);
            }
        }
        None
    }

    /// Select the best prefix to display a value in a given unit.
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
        ("", val_in_base)
    }

    /// Register a single unit at runtime (e.g., from `unit bread = 1`).
    pub fn add_runtime(&mut self, name: &str, dim: [i8; 7], scale: f64, offset: f64) {
        self.add(name, dim, scale, offset);
    }

    pub fn get(&self, name: &str) -> Option<&UnitDef> {
        self.units.get(name).and_then(|v| v.first())
    }
}

fn to_dim_array(v: &[i8]) -> [i8; 7] {
    let mut arr = [0i8; 7];
    for (i, &e) in v.iter().take(7).enumerate() {
        arr[i] = e;
    }
    arr
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
    fn resolve_common_units() {
        let reg = UnitRegistry::new();
        assert!(reg.resolve("min").is_some());
        assert!(reg.resolve("hr").is_some());
        assert!(reg.resolve("mph").is_some());
        assert!(reg.resolve("degC").is_some());
        assert!(reg.resolve("L").is_some());
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
