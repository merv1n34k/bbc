use crate::dim::DimVec;

#[derive(Debug, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("dimension mismatch: [{left}] != [{right}]")]
    DimensionMismatch {
        left: DimVec,
        right: DimVec,
        span: Option<Span>,
    },

    #[error("unknown unit: {name}")]
    UnknownUnit {
        name: String,
        span: Option<Span>,
    },

    #[error("unknown variable: {name}")]
    UnknownVariable {
        name: String,
        span: Option<Span>,
    },

    #[error("unknown function: {name}")]
    UnknownFunction {
        name: String,
        span: Option<Span>,
    },

    #[error("invalid arguments: {msg}")]
    InvalidArguments {
        msg: String,
        span: Option<Span>,
    },

    #[error("type error: {msg}")]
    TypeError {
        msg: String,
        span: Option<Span>,
    },

    #[error("parse error: {msg}")]
    ParseError {
        msg: String,
        span: Option<Span>,
    },

    #[error("invalid base literal: {msg}")]
    InvalidBaseLiteral {
        msg: String,
        span: Option<Span>,
    },

    #[error("division by zero")]
    DivisionByZero { span: Option<Span> },

    #[error("cannot take root: dimension {dim} not evenly divisible by {n}")]
    InvalidDimensionRoot {
        dim: DimVec,
        n: i8,
        span: Option<Span>,
    },
}

impl Error {
    pub fn span(&self) -> Option<&Span> {
        match self {
            Error::DimensionMismatch { span, .. }
            | Error::UnknownUnit { span, .. }
            | Error::UnknownVariable { span, .. }
            | Error::UnknownFunction { span, .. }
            | Error::InvalidArguments { span, .. }
            | Error::TypeError { span, .. }
            | Error::ParseError { span, .. }
            | Error::InvalidBaseLiteral { span, .. }
            | Error::DivisionByZero { span }
            | Error::InvalidDimensionRoot { span, .. } => span.as_ref(),
        }
    }
}
