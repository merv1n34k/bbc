/// A parsed unit expression like `km`, `m*s^-2`, `kg*m*s^-2`
#[derive(Debug, Clone)]
pub struct UnitExpr {
    pub parts: Vec<UnitPart>,
}

#[derive(Debug, Clone)]
pub struct UnitPart {
    pub name: String,
    pub exp: i8,
}

#[derive(Debug, Clone)]
pub enum Expr {
    /// Numeric literal (already parsed to string + base)
    Number { value: String, base: u32 },

    /// Boolean literal
    Bool(bool),

    /// String literal
    StringLit(String),

    /// Variable or constant reference
    Ident(String),

    /// Unary operation: -x, !x, ~x
    Unary { op: UnaryOp, expr: Box<Expr> },

    /// Binary operation: x + y, x * y, etc.
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    /// Function call: sin(x), det(A)
    Call { name: String, args: Vec<Expr> },

    /// Unit annotation: expr [unit]
    WithUnit { expr: Box<Expr>, unit: UnitExpr },

    /// Unit conversion and/or base conversion: expr -> [unit], expr -> 16x, expr -> 16x[unit]
    Convert {
        expr: Box<Expr>,
        target: Option<UnitExpr>,
        base: Option<u32>,
    },

    /// Variable assignment: x = expr
    Assign { name: String, expr: Box<Expr> },

    /// Constant assignment: const x = expr
    ConstAssign { name: String, expr: Box<Expr> },

    /// units command: units, units imperial, units +sci, units -imp
    UnitsCmd { action: UnitsCmdAction },

    /// unit command: unit x = expr, unit x, unit -x
    UnitCmd { action: UnitCmdAction },
}

#[derive(Debug, Clone)]
pub enum UnitsCmdAction {
    List,
    Load(String),
    Unload(String),
}

#[derive(Debug, Clone)]
pub enum UnitCmdAction {
    Define { name: String, expr: Box<Expr> },
    Inspect(String),
    Remove(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    // Bitwise
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Logical
    And,
    Or,
}

impl BinOp {
    pub fn precedence(self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::BitOr => 3,
            BinOp::BitXor => 4,
            BinOp::BitAnd => 5,
            BinOp::Eq | BinOp::Ne => 6,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => 7,
            BinOp::Shl | BinOp::Shr => 8,
            BinOp::Add | BinOp::Sub => 9,
            BinOp::Mul | BinOp::Div | BinOp::Mod => 10,
            BinOp::Pow => 11,
        }
    }
}
