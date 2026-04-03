use crate::dim::DimVec;
use malachite::Rational;

/// A label describing what unit a quantity is displayed in.
#[derive(Debug, Clone)]
pub struct UnitLabel {
    pub name: String,
    pub scale: Rational,
    pub offset: Rational,
    /// When true, the unit was explicitly requested via `->` conversion
    /// and should not be auto-prefixed.
    pub pinned: bool,
}

/// A quantity: number + dimension vector.
/// Dimensionless values have dim == DimVec::DIMENSIONLESS.
/// Internally, values are always in SI base units.
#[derive(Debug, Clone)]
pub struct Quantity {
    pub val: Rational,
    pub dim: DimVec,
    /// Preferred display unit (set by -> conversion or unit annotation).
    pub unit: Option<UnitLabel>,
    /// Override output base (set by -> 16x).
    pub display_base: Option<u32>,
    /// Significant figures (None = exact/infinite precision).
    pub sigfigs: Option<u32>,
}

impl Quantity {
    pub fn dimensionless(val: Rational) -> Self {
        Quantity {
            val,
            dim: DimVec::DIMENSIONLESS,
            unit: None,
            display_base: None,
            sigfigs: None,
        }
    }

    pub fn new(val: Rational, dim: DimVec) -> Self {
        Quantity { val, dim, unit: None, display_base: None, sigfigs: None }
    }

    pub fn with_unit(val: Rational, dim: DimVec, unit: UnitLabel) -> Self {
        Quantity {
            val,
            dim,
            unit: Some(unit),
            display_base: None,
            sigfigs: None,
        }
    }

    pub fn is_dimensionless(&self) -> bool {
        self.dim.is_dimensionless()
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Quantity(Quantity),
    Bool(bool),
    String(String),
}

impl Value {
    pub fn from_rational(val: Rational) -> Self {
        Value::Quantity(Quantity::dimensionless(val))
    }

    pub fn from_int(n: i64) -> Self {
        Value::from_rational(Rational::from(n))
    }

    pub fn as_quantity(&self) -> Option<&Quantity> {
        match self {
            Value::Quantity(q) => Some(q),
            _ => None,
        }
    }

    pub fn into_quantity(self) -> Option<Quantity> {
        match self {
            Value::Quantity(q) => Some(q),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Quantity(_) => "quantity",
            Value::Bool(_) => "bool",
            Value::String(_) => "string",
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Quantity(q) => {
                if q.dim.is_dimensionless() {
                    write!(f, "{}", q.val)
                } else {
                    write!(f, "{} [{}]", q.val, q.dim)
                }
            }
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
        }
    }
}
