/// Preprocess LaTeX notation into BBC syntax before lexing.
///
/// Transforms: \frac{a}{b} -> ((a)/(b)), \sqrt{x} -> sqrt(x),
/// \pi -> pi, \sin -> sin, x^{n} -> x^(n), etc.
pub fn preprocess_latex(input: &str) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\\' {
            let cmd_start = i + 1;
            let mut cmd_end = cmd_start;
            while cmd_end < len && bytes[cmd_end].is_ascii_alphabetic() {
                cmd_end += 1;
            }
            let cmd = &input[cmd_start..cmd_end];
            match cmd {
                "frac" => {
                    let (a, after_a) = read_brace_group(input, cmd_end);
                    let (b, after_b) = read_brace_group(input, after_a);
                    out.push_str(&format!("(({})/({})) ", a, b));
                    i = after_b;
                }
                "sqrt" => {
                    // \sqrt[n]{x} or \sqrt{x}
                    let j = skip_whitespace(input, cmd_end);
                    if j < len && bytes[j] == b'[' {
                        let (n, after_n) = read_bracket_group(input, j);
                        let (x, after_x) = read_brace_group(input, after_n);
                        if n == "3" {
                            out.push_str(&format!("cbrt({}) ", x));
                        } else {
                            out.push_str(&format!("({})^(1/{}) ", x, n));
                        }
                        i = after_x;
                    } else {
                        let (x, after_x) = read_brace_group(input, j);
                        out.push_str(&format!("sqrt({}) ", x));
                        i = after_x;
                    }
                }
                "ln" => {
                    let j = skip_whitespace(input, cmd_end);
                    if j < len && bytes[j] == b'{' {
                        let (x, after_x) = read_brace_group(input, j);
                        out.push_str(&format!("ln({}) ", x));
                        i = after_x;
                    } else {
                        out.push_str("ln");
                        i = cmd_end;
                    }
                }
                "log" => {
                    let j = skip_whitespace(input, cmd_end);
                    if j < len && bytes[j] == b'_' {
                        // \log_{b}{x}
                        let (b, after_b) = read_brace_group(input, j + 1);
                        let (x, after_x) = read_brace_group(input, after_b);
                        out.push_str(&format!("log({}, {}) ", x, b));
                        i = after_x;
                    } else if j < len && bytes[j] == b'{' {
                        let (x, after_x) = read_brace_group(input, j);
                        out.push_str(&format!("ln({}) ", x));
                        i = after_x;
                    } else {
                        out.push_str("ln");
                        i = cmd_end;
                    }
                }
                // Trig and other functions
                "sin" | "cos" | "tan" | "asin" | "acos" | "atan"
                | "sinh" | "cosh" | "tanh" | "exp" => {
                    let j = skip_whitespace(input, cmd_end);
                    if j < len && bytes[j] == b'{' {
                        let (x, after_x) = read_brace_group(input, j);
                        out.push_str(&format!("{}({}) ", cmd, x));
                        i = after_x;
                    } else {
                        out.push_str(cmd);
                        i = cmd_end;
                    }
                }
                // Greek letters -> identifiers
                "pi" => { out.push_str("pi"); i = cmd_end; }
                "tau" => { out.push_str("tau"); i = cmd_end; }
                "phi" => { out.push_str("phi"); i = cmd_end; }
                "alpha" => { out.push_str("alpha"); i = cmd_end; }
                "beta" => { out.push_str("beta"); i = cmd_end; }
                "gamma" => { out.push_str("gamma"); i = cmd_end; }
                "delta" => { out.push_str("delta"); i = cmd_end; }
                "epsilon" => { out.push_str("epsilon"); i = cmd_end; }
                "theta" => { out.push_str("theta"); i = cmd_end; }
                "lambda" => { out.push_str("lambda"); i = cmd_end; }
                "mu" => { out.push_str("mu"); i = cmd_end; }
                "sigma" => { out.push_str("sigma"); i = cmd_end; }
                "omega" => { out.push_str("omega"); i = cmd_end; }
                // Operators
                "cdot" | "times" => { out.push('*'); i = cmd_end; }
                "div" => { out.push('/'); i = cmd_end; }
                // Delimiters
                "left" => { i = cmd_end; }
                "right" => { i = cmd_end; }
                _ => {
                    // Unknown command: pass through as-is
                    out.push('\\');
                    out.push_str(cmd);
                    i = cmd_end;
                }
            }
        } else if bytes[i] == b'^' && i + 1 < len && bytes[i + 1] == b'{' {
            // x^{n} -> x^(n)
            let (n, after_n) = read_brace_group(input, i + 1);
            out.push_str(&format!("^({}) ", n));
            i = after_n;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }

    out
}

fn skip_whitespace(input: &str, start: usize) -> usize {
    let bytes = input.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Read a {braced} group, returning (contents, position_after_closing_brace).
/// If no opening brace found, returns empty string at start position.
fn read_brace_group(input: &str, start: usize) -> (String, usize) {
    let j = skip_whitespace(input, start);
    let bytes = input.as_bytes();
    if j >= bytes.len() || bytes[j] != b'{' {
        return (String::new(), start);
    }
    let mut depth = 1;
    let mut k = j + 1;
    while k < bytes.len() && depth > 0 {
        if bytes[k] == b'{' {
            depth += 1;
        } else if bytes[k] == b'}' {
            depth -= 1;
        }
        k += 1;
    }
    let content = &input[j + 1..k - 1];
    (preprocess_latex(content), k)
}

/// Read a [bracketed] group, returning (contents, position_after_closing_bracket).
fn read_bracket_group(input: &str, start: usize) -> (String, usize) {
    let bytes = input.as_bytes();
    if start >= bytes.len() || bytes[start] != b'[' {
        return (String::new(), start);
    }
    let mut depth = 1;
    let mut k = start + 1;
    while k < bytes.len() && depth > 0 {
        if bytes[k] == b'[' {
            depth += 1;
        } else if bytes[k] == b']' {
            depth -= 1;
        }
        k += 1;
    }
    let content = &input[start + 1..k - 1];
    (content.to_string(), k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frac() {
        let result = preprocess_latex(r"\frac{1}{3}");
        assert!(result.contains("((1)/(3))"));
    }

    #[test]
    fn sqrt_simple() {
        let result = preprocess_latex(r"\sqrt{144}");
        assert!(result.contains("sqrt(144)"));
    }

    #[test]
    fn sqrt_nth() {
        let result = preprocess_latex(r"\sqrt[3]{27}");
        assert!(result.contains("cbrt(27)"));
    }

    #[test]
    fn sqrt_4th() {
        let result = preprocess_latex(r"\sqrt[4]{16}");
        assert!(result.contains("(16)^(1/4)"));
    }

    #[test]
    fn trig() {
        assert!(preprocess_latex(r"\sin{x}").contains("sin(x)"));
        assert!(preprocess_latex(r"\cos{x}").contains("cos(x)"));
    }

    #[test]
    fn greek_letters() {
        assert!(preprocess_latex(r"\pi").contains("pi"));
        assert!(preprocess_latex(r"\tau").contains("tau"));
    }

    #[test]
    fn operators() {
        assert!(preprocess_latex(r"a \cdot b").contains("a * b"));
        assert!(preprocess_latex(r"a \times b").contains("a * b"));
        assert!(preprocess_latex(r"a \div b").contains("a / b"));
    }

    #[test]
    fn power_braces() {
        let result = preprocess_latex(r"x^{2}");
        assert!(result.contains("^(2)"));
    }

    #[test]
    fn nested_frac() {
        let result = preprocess_latex(r"\frac{1}{\frac{2}{3}}");
        assert!(result.contains("((1)/(((2)/(3))"))
    }

    #[test]
    fn log_with_base() {
        let result = preprocess_latex(r"\log_{2}{8}");
        assert!(result.contains("log(8, 2)"));
    }

    #[test]
    fn delimiters_stripped() {
        let result = preprocess_latex(r"\left(\frac{1}{2}\right)");
        assert!(result.contains("(((1)/(2))"));
        assert!(!result.contains("left"));
        assert!(!result.contains("right"));
    }
}
