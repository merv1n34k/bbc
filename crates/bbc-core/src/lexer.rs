use crate::error::{Error, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Decimal number: "42", "3.14", "1.5e-3"
    Number(String),
    /// Base-prefixed number: (base, digits) e.g. (16, "FF") for 16xFF
    BasedNumber(u32, String),
    /// Identifier: variable/function name
    Ident(String),
    /// Boolean literal
    Bool(bool),
    /// String literal
    StringLit(String),

    // Arithmetic
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,

    CaretCaret,

    // Bitwise
    Ampersand,
    Pipe,
    Tilde,
    Shl,
    Shr,

    // Comparison
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    // Logical
    AmpAmp,
    PipePipe,
    Bang,

    // Assignment and conversion
    Eq,
    Arrow, // ->

    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Semicolon,

    Eof,
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

pub fn lex(input: &str) -> Result<Vec<SpannedToken>, Error> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Line comments
        if bytes[i] == b'#' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        let start = i;

        // String literal
        if bytes[i] == b'"' {
            i += 1;
            let mut s = String::new();
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 1;
                    match bytes[i] {
                        b'n' => s.push('\n'),
                        b't' => s.push('\t'),
                        b'\\' => s.push('\\'),
                        b'"' => s.push('"'),
                        _ => {
                            s.push('\\');
                            s.push(bytes[i] as char);
                        }
                    }
                } else {
                    s.push(bytes[i] as char);
                }
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
            tokens.push(SpannedToken {
                token: Token::StringLit(s),
                span: Span { start, end: i },
            });
            continue;
        }

        // Numbers
        if bytes[i].is_ascii_digit()
            || (bytes[i] == b'.' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit())
        {
            let (tok, end) = lex_number(input, i)?;
            tokens.push(SpannedToken {
                token: tok,
                span: Span { start, end },
            });
            i = end;
            continue;
        }

        // Identifiers and keywords
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let end = scan_while(bytes, i, |b| b.is_ascii_alphanumeric() || b == b'_');
            let word = &input[i..end];
            let token = match word {
                "true" => Token::Bool(true),
                "false" => Token::Bool(false),
                _ => Token::Ident(word.to_string()),
            };
            tokens.push(SpannedToken {
                token,
                span: Span { start, end },
            });
            i = end;
            continue;
        }

        // Two-char operators
        if i + 1 < bytes.len() {
            let two = &input[i..i + 2];
            let tok = match two {
                "->" => Some(Token::Arrow),
                "==" => Some(Token::EqEq),
                "!=" => Some(Token::BangEq),
                "<=" => Some(Token::LtEq),
                ">=" => Some(Token::GtEq),
                "<<" => Some(Token::Shl),
                ">>" => Some(Token::Shr),
                "&&" => Some(Token::AmpAmp),
                "||" => Some(Token::PipePipe),
                "^^" => Some(Token::CaretCaret),
                _ => None,
            };
            if let Some(tok) = tok {
                tokens.push(SpannedToken {
                    token: tok,
                    span: Span { start, end: i + 2 },
                });
                i += 2;
                continue;
            }
        }

        // Single-char operators
        let tok = match bytes[i] {
            b'+' => Some(Token::Plus),
            b'-' => Some(Token::Minus),
            b'*' => Some(Token::Star),
            b'/' => Some(Token::Slash),
            b'%' => Some(Token::Percent),
            b'^' => Some(Token::Caret),
            b'&' => Some(Token::Ampersand),
            b'|' => Some(Token::Pipe),
            b'~' => Some(Token::Tilde),
            b'!' => Some(Token::Bang),
            b'<' => Some(Token::Lt),
            b'>' => Some(Token::Gt),
            b'=' => Some(Token::Eq),
            b'(' => Some(Token::LParen),
            b')' => Some(Token::RParen),
            b'[' => Some(Token::LBracket),
            b']' => Some(Token::RBracket),
            b',' => Some(Token::Comma),
            b';' => Some(Token::Semicolon),
            _ => None,
        };

        if let Some(tok) = tok {
            tokens.push(SpannedToken {
                token: tok,
                span: Span { start, end: i + 1 },
            });
            i += 1;
            continue;
        }

        return Err(Error::ParseError {
            msg: format!("unexpected character: '{}'", bytes[i] as char),
            span: Some(Span { start, end: i + 1 }),
        });
    }

    tokens.push(SpannedToken {
        token: Token::Eof,
        span: Span {
            start: input.len(),
            end: input.len(),
        },
    });

    Ok(tokens)
}

fn lex_number(input: &str, start: usize) -> Result<(Token, usize), Error> {
    let bytes = input.as_bytes();
    let mut i = start;

    // Scan initial digits
    let digits_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }

    // Check for base prefix: <digits>x<alphanumeric>
    if i < bytes.len() && bytes[i] == b'x' && i > digits_start {
        let base_str = &input[digits_start..i];
        if let Ok(base) = base_str.parse::<u32>() {
            if base >= 2
                && base <= 36
                && i + 1 < bytes.len()
                && is_base_digit(bytes[i + 1], base)
            {
                i += 1; // skip 'x'
                let dstart = i;
                while i < bytes.len() && is_base_digit(bytes[i], base) {
                    i += 1;
                }
                let digits = input[dstart..i].to_uppercase();
                for ch in digits.chars() {
                    let d = if ch.is_ascii_digit() {
                        ch as u32 - '0' as u32
                    } else {
                        ch as u32 - 'A' as u32 + 10
                    };
                    if d >= base {
                        return Err(Error::InvalidBaseLiteral {
                            msg: format!("digit '{}' invalid for base {}", ch, base),
                            span: Some(Span { start, end: i }),
                        });
                    }
                }
                return Ok((Token::BasedNumber(base, digits), i));
            }
        }
    }

    // Decimal point
    if i < bytes.len() && bytes[i] == b'.' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit()
    {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }

    // Scientific notation
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }

    let num_str = input[start..i].to_string();
    Ok((Token::Number(num_str), i))
}

fn is_base_digit(b: u8, base: u32) -> bool {
    let d = if b.is_ascii_digit() {
        (b - b'0') as u32
    } else if b.is_ascii_uppercase() {
        (b - b'A') as u32 + 10
    } else if b.is_ascii_lowercase() {
        (b - b'a') as u32 + 10
    } else {
        return false;
    };
    d < base
}

fn scan_while(bytes: &[u8], start: usize, pred: impl Fn(u8) -> bool) -> usize {
    let mut i = start;
    while i < bytes.len() && pred(bytes[i]) {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(input: &str) -> Vec<Token> {
        lex(input).unwrap().into_iter().map(|t| t.token).collect()
    }

    #[test]
    fn basic_number() {
        assert_eq!(toks("42"), vec![Token::Number("42".into()), Token::Eof]);
    }

    #[test]
    fn hex_number() {
        assert_eq!(
            toks("16xFF"),
            vec![Token::BasedNumber(16, "FF".into()), Token::Eof]
        );
    }

    #[test]
    fn binary_number() {
        assert_eq!(
            toks("2x1010"),
            vec![Token::BasedNumber(2, "1010".into()), Token::Eof]
        );
    }

    #[test]
    fn decimal_point() {
        assert_eq!(
            toks("3.14"),
            vec![Token::Number("3.14".into()), Token::Eof]
        );
    }

    #[test]
    fn scientific() {
        assert_eq!(
            toks("1.5e-3"),
            vec![Token::Number("1.5e-3".into()), Token::Eof]
        );
    }

    #[test]
    fn expression() {
        assert_eq!(
            toks("2 + 3 * 4"),
            vec![
                Token::Number("2".into()),
                Token::Plus,
                Token::Number("3".into()),
                Token::Star,
                Token::Number("4".into()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn unit_annotation() {
        assert_eq!(
            toks("5 [kg]"),
            vec![
                Token::Number("5".into()),
                Token::LBracket,
                Token::Ident("kg".into()),
                Token::RBracket,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn conversion() {
        assert_eq!(
            toks("100 [km] -> [mi]"),
            vec![
                Token::Number("100".into()),
                Token::LBracket,
                Token::Ident("km".into()),
                Token::RBracket,
                Token::Arrow,
                Token::LBracket,
                Token::Ident("mi".into()),
                Token::RBracket,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn booleans() {
        assert_eq!(
            toks("true && false"),
            vec![Token::Bool(true), Token::AmpAmp, Token::Bool(false), Token::Eof]
        );
    }

    #[test]
    fn comment() {
        assert_eq!(toks("42 # comment"), vec![Token::Number("42".into()), Token::Eof]);
    }
}
