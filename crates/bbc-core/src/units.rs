use std::collections::HashMap;

use serde::Deserialize;

use crate::dim::DimVec;

include!(concat!(env!("OUT_DIR"), "/unit_sets.rs"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitSource {
    Si,
    Common,
    Set(String),
    Runtime,
}

#[derive(Debug, Clone)]
pub struct UnitDef {
    pub name: String,
    pub dim: DimVec,
    pub scale: f64,
    pub offset: f64,
    pub source: UnitSource,
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
struct TomlUnitDef {
    scale: f64,
    dim: Vec<i8>,
    #[serde(default)]
    offset: f64,
    #[serde(default)]
    alias: bool,
}

#[derive(Deserialize)]
struct TomlConstantDef {
    value: f64,
    dim: Vec<i8>,
}

pub struct ParsedConstant {
    pub name: String,
    pub value: f64,
    pub dim: DimVec,
}

pub struct UnitRegistry {
    units: HashMap<String, Vec<UnitDef>>,
    derived_names: Vec<(DimVec, String)>,
    loaded_sets: Vec<String>,
}

impl UnitRegistry {
    pub fn new() -> Self {
        let mut reg = UnitRegistry {
            units: HashMap::new(),
            derived_names: Vec::new(),
            loaded_sets: Vec::new(),
        };
        reg.register_si_base();
        // Load [common] sections from ALL data files
        for &(_name, content) in UNIT_SETS {
            reg.load_common(content);
        }
        reg
    }

    fn register_si_base(&mut self) {
        self.add_with_source("m",   [1, 0, 0, 0, 0, 0, 0], 1.0, 0.0, UnitSource::Si);
        self.add_with_source("g",   [0, 1, 0, 0, 0, 0, 0], 1e-3, 0.0, UnitSource::Si);
        self.add_with_source("s",   [0, 0, 1, 0, 0, 0, 0], 1.0, 0.0, UnitSource::Si);
        self.add_with_source("A",   [0, 0, 0, 1, 0, 0, 0], 1.0, 0.0, UnitSource::Si);
        self.add_with_source("K",   [0, 0, 0, 0, 1, 0, 0], 1.0, 0.0, UnitSource::Si);
        self.add_with_source("mol", [0, 0, 0, 0, 0, 1, 0], 1.0, 0.0, UnitSource::Si);
        self.add_with_source("cd",  [0, 0, 0, 0, 0, 0, 1], 1.0, 0.0, UnitSource::Si);
    }

    fn add_with_source(&mut self, name: &str, dim: [i8; 7], scale: f64, offset: f64, source: UnitSource) {
        let def = UnitDef {
            name: name.to_string(),
            dim: DimVec::new(dim),
            scale,
            offset,
            source,
        };
        self.units.entry(name.to_string()).or_default().push(def);
    }

    /// Load [common] section from a TOML file (always loaded).
    fn load_common(&mut self, content: &str) {
        let table: toml::Table = toml::from_str(content)
            .expect("failed to parse TOML");
        if let Some(toml::Value::Table(common)) = table.get("common") {
            let defs = parse_unit_table(common);
            self.register_unit_defs(&defs, UnitSource::Common);
        }
    }

    /// Load a named unit set (e.g., "imperial", "scientific").
    /// Searches all TOML files for a section with that name.
    /// Idempotent: loading an already-loaded set is a no-op.
    pub fn load_unit_set(&mut self, name: &str) {
        if self.loaded_sets.iter().any(|s| s == name) {
            return;
        }
        for &(_file_name, content) in UNIT_SETS {
            let table: toml::Table = toml::from_str(content)
                .expect("failed to parse TOML");
            if let Some(toml::Value::Table(section)) = table.get(name) {
                let defs = parse_unit_table(section);
                self.register_unit_defs(&defs, UnitSource::Set(name.to_string()));
                self.loaded_sets.push(name.to_string());
                return;
            }
        }
        eprintln!("warning: unknown unit set '{}', available: {:?}",
            name, Self::available_unit_sets());
    }

    fn register_unit_defs(&mut self, defs: &[(String, TomlUnitDef)], source: UnitSource) {
        for (name, def) in defs {
            let dim_arr = to_dim_array(&def.dim);
            let dv = DimVec::new(dim_arr);
            let unit_def = UnitDef {
                name: name.clone(),
                dim: dv,
                scale: def.scale,
                offset: def.offset,
                source: source.clone(),
            };

            let entries = self.units.entry(name.clone()).or_default();
            if let Some(existing) = entries.iter_mut().find(|e| e.dim == dv) {
                eprintln!("warning: duplicate unit '{}' ({}) redefined, using last definition", name, dv);
                *existing = unit_def;
            } else {
                entries.push(unit_def);
            }

            if def.alias && !self.derived_names.iter().any(|(d, _)| *d == dv) {
                self.derived_names.push((dv, name.clone()));
            }
        }
    }

    /// Load [constants] from all embedded TOML files.
    pub fn load_constants() -> Vec<ParsedConstant> {
        let mut result = Vec::new();
        for &(_name, content) in UNIT_SETS {
            let table: toml::Table = toml::from_str(content)
                .expect("failed to parse TOML");
            if let Some(toml::Value::Table(constants)) = table.get("constants") {
                for (name, value) in constants {
                    let def: TomlConstantDef = value.clone().try_into()
                        .unwrap_or_else(|e| panic!("bad constant '{}': {}", name, e));
                    result.push(ParsedConstant {
                        name: name.clone(),
                        value: def.value,
                        dim: DimVec::new(to_dim_array(&def.dim)),
                    });
                }
            }
        }
        result
    }

    /// Returns available unit set names (sections that are not "common" or "constants").
    pub fn available_unit_sets() -> Vec<String> {
        let mut sets = Vec::new();
        for &(_file_name, content) in UNIT_SETS {
            let table: toml::Table = toml::from_str(content)
                .expect("failed to parse TOML");
            for (key, value) in &table {
                if key != "common" && key != "constants"
                    && value.is_table() && !sets.contains(key)
                {
                    sets.push(key.clone());
                }
            }
        }
        sets
    }

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

    /// Given a unit name, return (base_name, prefix_exponent) if the name
    /// is a prefixed form of a registered unit, or (name, 0) if it's a direct
    /// registered unit. Returns None if the name isn't resolvable.
    pub fn base_unit_name<'a>(&self, name: &'a str) -> Option<(&'a str, i8)> {
        if self.units.contains_key(name) {
            return Some((name, 0));
        }
        for prefix in PREFIXES {
            if let Some(base) = name.strip_prefix(prefix.symbol) {
                if !base.is_empty() && self.units.contains_key(base) {
                    return Some((base, prefix.exponent));
                }
            }
        }
        None
    }

    pub fn find_derived_name(&self, dim: DimVec) -> Option<&str> {
        for (d, name) in &self.derived_names {
            if *d == dim {
                return Some(name);
            }
        }
        None
    }

    pub fn best_prefix(val_in_base: f64) -> (&'static str, f64) {
        let abs = val_in_base.abs();
        if abs == 0.0 {
            return ("", val_in_base);
        }
        for &(sym, exp) in DISPLAY_PREFIXES {
            let scale = 10f64.powi(exp as i32);
            let scaled = abs / scale;
            if (1.0..1000.0).contains(&scaled) {
                return (sym, val_in_base / scale);
            }
        }
        ("", val_in_base)
    }

    /// Add or update a runtime unit definition. Idempotent: redefining
    /// the same name overwrites the previous runtime definition.
    pub fn add_runtime(&mut self, name: &str, dim: [i8; 7], scale: f64, offset: f64) {
        let dv = DimVec::new(dim);
        let def = UnitDef {
            name: name.to_string(),
            dim: dv,
            scale,
            offset,
            source: UnitSource::Runtime,
        };
        let entries = self.units.entry(name.to_string()).or_default();
        if let Some(existing) = entries.iter_mut().find(|e| e.source == UnitSource::Runtime) {
            *existing = def;
        } else {
            entries.push(def);
        }
    }

    /// Unload a named unit set, removing all units that came from it.
    /// Idempotent: unloading a set that isn't loaded is a no-op.
    pub fn unload_unit_set(&mut self, name: &str) {
        self.loaded_sets.retain(|s| s != name);
        let target = UnitSource::Set(name.to_string());
        for entries in self.units.values_mut() {
            entries.retain(|d| d.source != target);
        }
        self.units.retain(|_, v| !v.is_empty());
    }

    /// Remove a runtime unit by name. Returns true if something was removed.
    /// Idempotent: removing a non-existent unit returns false.
    pub fn remove_runtime_unit(&mut self, name: &str) -> bool {
        if let Some(entries) = self.units.get_mut(name) {
            let before = entries.len();
            entries.retain(|d| d.source != UnitSource::Runtime);
            let removed = entries.len() < before;
            if entries.is_empty() {
                self.units.remove(name);
            }
            removed
        } else {
            false
        }
    }

    pub fn loaded_sets(&self) -> &[String] {
        &self.loaded_sets
    }

    pub fn describe_unit(&self, name: &str) -> Option<String> {
        if let Some(defs) = self.units.get(name) {
            if let Some(def) = defs.first() {
                let source = match &def.source {
                    UnitSource::Si => "SI base",
                    UnitSource::Common => "common",
                    UnitSource::Set(s) => s.as_str(),
                    UnitSource::Runtime => "runtime",
                };
                return Some(format!(
                    "{}: scale={}, dim={}, source={}",
                    def.name, def.scale, def.dim, source
                ));
            }
        }
        // Check with prefix
        for prefix in PREFIXES {
            if let Some(base) = name.strip_prefix(prefix.symbol) {
                if !base.is_empty() {
                    if let Some(defs) = self.units.get(base) {
                        if let Some(def) = defs.first() {
                            let prefix_scale = 10f64.powi(prefix.exponent as i32);
                            return Some(format!(
                                "{}: scale={} ({}*{}), dim={}",
                                name, def.scale * prefix_scale, prefix.symbol, def.name, def.dim
                            ));
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get(&self, name: &str) -> Option<&UnitDef> {
        self.units.get(name).and_then(|v| v.first())
    }
}

fn parse_unit_table(table: &toml::map::Map<String, toml::Value>) -> Vec<(String, TomlUnitDef)> {
    let mut result = Vec::new();
    for (k, v) in table {
        if let toml::Value::Array(arr) = v {
            for item in arr {
                let def: TomlUnitDef = item.clone().try_into()
                    .unwrap_or_else(|e| panic!("bad unit def '{}': {}", k, e));
                result.push((k.clone(), def));
            }
        } else {
            let def: TomlUnitDef = v.clone().try_into()
                .unwrap_or_else(|e| panic!("bad unit def '{}': {}", k, e));
            result.push((k.clone(), def));
        }
    }
    result
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
