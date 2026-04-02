use crate::ast::*;
use crate::error::{Error, Span};
use crate::lexer::{SpannedToken, Token};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Expr, Error> {
        let mut expr = self.parse_expr(0)?;

        // -> at lowest precedence: checked after the full expression
        if matches!(self.peek(), Token::Arrow) {
            self.advance();
            let (base, target) = self.parse_conversion_target()?;
            expr = Expr::Convert {
                expr: Box::new(expr),
                target,
                base,
            };
        }

        if !self.at_eof() {
            let tok = &self.tokens[self.pos];
            return Err(Error::ParseError {
                msg: format!("unexpected token: {:?}", tok.token),
                span: Some(tok.span.clone()),
            });
        }
        Ok(expr)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    fn peek_span(&self) -> &Span {
        &self.tokens[self.pos].span
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }

    fn advance(&mut self) -> &SpannedToken {
        let tok = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<&SpannedToken, Error> {
        if self.peek() == expected {
            Ok(self.advance())
        } else {
            Err(Error::ParseError {
                msg: format!("expected {:?}, got {:?}", expected, self.peek()),
                span: Some(self.peek_span().clone()),
            })
        }
    }

    /// Pratt parser: parse expression with minimum precedence `min_prec`.
    fn parse_expr(&mut self, min_prec: u8) -> Result<Expr, Error> {
        let mut left = self.parse_unary()?;

        loop {
            // Check for unit annotation: expr [unit]
            if matches!(self.peek(), Token::LBracket) {
                // Look ahead to see if this is a unit annotation
                let saved = self.pos;
                if let Ok(unit_expr) = self.try_parse_unit_annotation() {
                    left = Expr::WithUnit {
                        expr: Box::new(left),
                        unit: unit_expr,
                    };
                    continue;
                } else {
                    self.pos = saved;
                }
            }

            // Check for binary operator
            if let Some((op, prec)) = self.peek_binop() {
                if prec < min_prec {
                    break;
                }
                self.advance();
                // Right-associative for Pow
                let next_prec = if op == BinOp::Pow { prec } else { prec + 1 };
                let right = self.parse_expr(next_prec)?;
                left = Expr::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                };
                continue;
            }

            break;
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, Error> {
        match self.peek().clone() {
            Token::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                })
            }
            Token::Bang => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                })
            }
            Token::Tilde => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::BitNot,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, Error> {
        match self.peek().clone() {
            Token::Units => {
                self.advance();
                let action = self.parse_units_action()?;
                Ok(Expr::UnitsCmd { action })
            }
            Token::Unit => {
                self.advance();
                let action = self.parse_unit_action()?;
                Ok(Expr::UnitCmd { action })
            }
            Token::Const => {
                self.advance();
                if let Token::Ident(name) = self.peek().clone() {
                    self.advance();
                    self.expect(&Token::Eq)?;
                    let expr = self.parse_expr(0)?;
                    return Ok(Expr::ConstAssign {
                        name,
                        expr: Box::new(expr),
                    });
                }
                Err(Error::ParseError {
                    msg: "expected identifier after 'const'".to_string(),
                    span: Some(self.peek_span().clone()),
                })
            }
            Token::Number(s) => {
                self.advance();
                Ok(Expr::Number {
                    value: s,
                    base: 10,
                })
            }
            Token::BasedNumber(base, digits) => {
                self.advance();
                Ok(Expr::Number {
                    value: digits,
                    base,
                })
            }
            Token::Bool(b) => {
                self.advance();
                Ok(Expr::Bool(b))
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(Expr::StringLit(s))
            }
            Token::Ident(name) => {
                self.advance();
                // Check for function call: ident(...)
                if matches!(self.peek(), Token::LParen) {
                    self.advance(); // consume '('
                    let args = self.parse_args()?;
                    self.expect(&Token::RParen)?;
                    return Ok(Expr::Call { name, args });
                }
                // Check for assignment: ident = expr
                if matches!(self.peek(), Token::Eq) {
                    // Make sure it's not == (already handled by lexer as EqEq)
                    self.advance(); // consume '='
                    let expr = self.parse_expr(0)?;
                    return Ok(Expr::Assign {
                        name,
                        expr: Box::new(expr),
                    });
                }
                Ok(Expr::Ident(name))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr(0)?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            _ => Err(Error::ParseError {
                msg: format!("unexpected token: {:?}", self.peek()),
                span: Some(self.peek_span().clone()),
            }),
        }
    }

    fn parse_units_action(&mut self) -> Result<UnitsCmdAction, Error> {
        if self.at_eof() {
            return Ok(UnitsCmdAction::List);
        }
        // units -name -> Unload
        if matches!(self.peek(), Token::Minus) {
            self.advance();
            if let Token::Ident(name) = self.peek().clone() {
                self.advance();
                return Ok(UnitsCmdAction::Unload(name));
            }
            return Err(Error::ParseError {
                msg: "expected set name after '-'".to_string(),
                span: Some(self.peek_span().clone()),
            });
        }
        // units +name -> Load
        if matches!(self.peek(), Token::Plus) {
            self.advance();
            if let Token::Ident(name) = self.peek().clone() {
                self.advance();
                return Ok(UnitsCmdAction::Load(name));
            }
            return Err(Error::ParseError {
                msg: "expected set name after '+'".to_string(),
                span: Some(self.peek_span().clone()),
            });
        }
        // units name -> Load
        if let Token::Ident(name) = self.peek().clone() {
            self.advance();
            return Ok(UnitsCmdAction::Load(name));
        }
        Ok(UnitsCmdAction::List)
    }

    fn parse_unit_action(&mut self) -> Result<UnitCmdAction, Error> {
        // unit -name -> Remove
        if matches!(self.peek(), Token::Minus) {
            self.advance();
            if let Token::Ident(name) = self.peek().clone() {
                self.advance();
                return Ok(UnitCmdAction::Remove(name));
            }
            return Err(Error::ParseError {
                msg: "expected unit name after '-'".to_string(),
                span: Some(self.peek_span().clone()),
            });
        }
        // unit name ...
        if let Token::Ident(name) = self.peek().clone() {
            self.advance();
            // unit name = expr -> Define
            if matches!(self.peek(), Token::Eq) {
                self.advance();
                let expr = self.parse_expr(0)?;
                return Ok(UnitCmdAction::Define {
                    name,
                    expr: Box::new(expr),
                });
            }
            // unit name -> Inspect
            return Ok(UnitCmdAction::Inspect(name));
        }
        Err(Error::ParseError {
            msg: "expected unit name after 'unit'".to_string(),
            span: Some(self.peek_span().clone()),
        })
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, Error> {
        let mut args = Vec::new();
        if matches!(self.peek(), Token::RParen) {
            return Ok(args);
        }
        args.push(self.parse_expr(0)?);
        while matches!(self.peek(), Token::Comma) {
            self.advance();
            args.push(self.parse_expr(0)?);
        }
        Ok(args)
    }

    /// Parse conversion target after `->`: `[unit]`, `16x`, or `16x[unit]`.
    fn parse_conversion_target(&mut self) -> Result<(Option<u32>, Option<UnitExpr>), Error> {
        let mut base = None;
        let mut target = None;

        // Check for base format: Number followed by Ident("x")
        if let Token::Number(ref n) = self.peek().clone() {
            let saved = self.pos;
            let n_val = n.clone();
            self.advance();
            if let Token::Ident(ref s) = self.peek().clone() {
                if s == "x" {
                    let b: u32 = n_val.parse().map_err(|_| Error::ParseError {
                        msg: format!("invalid base: {}", n_val),
                        span: Some(self.peek_span().clone()),
                    })?;
                    if !(2..=36).contains(&b) {
                        return Err(Error::ParseError {
                            msg: format!("base must be 2-36, got {}", b),
                            span: Some(self.peek_span().clone()),
                        });
                    }
                    base = Some(b);
                    self.advance(); // consume "x"
                } else {
                    self.pos = saved;
                }
            } else {
                self.pos = saved;
            }
        }

        // Check for unit: [unit]
        if matches!(self.peek(), Token::LBracket) {
            self.advance();
            target = Some(self.parse_unit_expr()?);
            self.expect(&Token::RBracket)?;
        }

        if base.is_none() && target.is_none() {
            return Err(Error::ParseError {
                msg: "expected base format (e.g., 16x) or unit (e.g., [km]) after '->'".to_string(),
                span: Some(self.peek_span().clone()),
            });
        }

        Ok((base, target))
    }

    fn try_parse_unit_annotation(&mut self) -> Result<UnitExpr, Error> {
        self.expect(&Token::LBracket)?;
        let unit = self.parse_unit_expr()?;
        self.expect(&Token::RBracket)?;
        Ok(unit)
    }

    /// Parse unit expression inside brackets: `m*s^-2`, `kg`, `km/h`
    fn parse_unit_expr(&mut self) -> Result<UnitExpr, Error> {
        let mut parts = Vec::new();
        let mut negate_next = false;

        loop {
            match self.peek().clone() {
                Token::Ident(name) => {
                    self.advance();
                    let mut exp: i8 = if negate_next { -1 } else { 1 };

                    // Check for ^N exponent
                    if matches!(self.peek(), Token::Caret) {
                        self.advance();
                        let neg = matches!(self.peek(), Token::Minus);
                        if neg {
                            self.advance();
                        }
                        if let Token::Number(n) = self.peek().clone() {
                            self.advance();
                            let e: i8 = n
                                .parse()
                                .map_err(|_| Error::ParseError {
                                    msg: format!("invalid unit exponent: {}", n),
                                    span: Some(self.peek_span().clone()),
                                })?;
                            exp = if neg { -e } else { e };
                        }
                    }

                    parts.push(UnitPart { name, exp });
                }
                _ => break,
            }

            // Check for * or /
            match self.peek() {
                Token::Star => {
                    self.advance();
                    negate_next = false;
                }
                Token::Slash => {
                    self.advance();
                    negate_next = true;
                }
                _ => break,
            }
        }

        if parts.is_empty() {
            return Err(Error::ParseError {
                msg: "expected unit name".to_string(),
                span: Some(self.peek_span().clone()),
            });
        }

        Ok(UnitExpr { parts })
    }

    fn peek_binop(&self) -> Option<(BinOp, u8)> {
        let op = match self.peek() {
            Token::Plus => BinOp::Add,
            Token::Minus => BinOp::Sub,
            Token::Star => BinOp::Mul,
            Token::Slash => BinOp::Div,
            Token::Percent => BinOp::Mod,
            Token::Caret => BinOp::Pow,
            Token::Ampersand => BinOp::BitAnd,
            Token::Pipe => BinOp::BitOr,
            Token::CaretCaret => BinOp::BitXor,
            Token::Shl => BinOp::Shl,
            Token::Shr => BinOp::Shr,
            Token::EqEq => BinOp::Eq,
            Token::BangEq => BinOp::Ne,
            Token::Lt => BinOp::Lt,
            Token::LtEq => BinOp::Le,
            Token::Gt => BinOp::Gt,
            Token::GtEq => BinOp::Ge,
            Token::AmpAmp => BinOp::And,
            Token::PipePipe => BinOp::Or,
            _ => return None,
        };
        Some((op, op.precedence()))
    }
}

pub fn parse(input: &str) -> Result<Expr, Error> {
    let tokens = crate::lexer::lex(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_number() {
        let expr = parse("42").unwrap();
        assert!(matches!(expr, Expr::Number { base: 10, .. }));
    }

    #[test]
    fn parse_hex() {
        let expr = parse("16xFF").unwrap();
        assert!(matches!(expr, Expr::Number { base: 16, .. }));
    }

    #[test]
    fn parse_binary_op() {
        let expr = parse("2 + 3").unwrap();
        assert!(matches!(expr, Expr::Binary { op: BinOp::Add, .. }));
    }

    #[test]
    fn parse_precedence() {
        // 2 + 3 * 4 should parse as 2 + (3 * 4)
        let expr = parse("2 + 3 * 4").unwrap();
        match expr {
            Expr::Binary {
                op: BinOp::Add,
                right,
                ..
            } => {
                assert!(matches!(*right, Expr::Binary { op: BinOp::Mul, .. }));
            }
            _ => panic!("expected Add at top level"),
        }
    }

    #[test]
    fn parse_unary_neg() {
        let expr = parse("-5").unwrap();
        assert!(matches!(
            expr,
            Expr::Unary {
                op: UnaryOp::Neg,
                ..
            }
        ));
    }

    #[test]
    fn parse_function_call() {
        let expr = parse("sin(3.14)").unwrap();
        match expr {
            Expr::Call { name, args } => {
                assert_eq!(name, "sin");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("expected Call"),
        }
    }

    #[test]
    fn parse_unit_annotation() {
        let expr = parse("5 [kg]").unwrap();
        assert!(matches!(expr, Expr::WithUnit { .. }));
    }

    #[test]
    fn parse_unit_conversion() {
        let expr = parse("100 [km] -> [mi]").unwrap();
        assert!(matches!(expr, Expr::Convert { .. }));
    }

    #[test]
    fn parse_compound_unit() {
        let expr = parse("9.8 [m*s^-2]").unwrap();
        match expr {
            Expr::WithUnit { unit, .. } => {
                assert_eq!(unit.parts.len(), 2);
                assert_eq!(unit.parts[0].name, "m");
                assert_eq!(unit.parts[0].exp, 1);
                assert_eq!(unit.parts[1].name, "s");
                assert_eq!(unit.parts[1].exp, -2);
            }
            _ => panic!("expected WithUnit"),
        }
    }

    #[test]
    fn parse_assignment() {
        let expr = parse("x = 42").unwrap();
        assert!(matches!(expr, Expr::Assign { .. }));
    }

    #[test]
    fn parse_parens() {
        let expr = parse("(2 + 3) * 4").unwrap();
        assert!(matches!(expr, Expr::Binary { op: BinOp::Mul, .. }));
    }

    #[test]
    fn parse_power_right_assoc() {
        // 2^3^4 should parse as 2^(3^4)
        let expr = parse("2 ^ 3 ^ 4").unwrap();
        match expr {
            Expr::Binary {
                op: BinOp::Pow,
                right,
                ..
            } => {
                assert!(matches!(*right, Expr::Binary { op: BinOp::Pow, .. }));
            }
            _ => panic!("expected Pow at top level"),
        }
    }
}
