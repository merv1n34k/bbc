mod repl;

use bbc_core::env::Env;
use bbc_core::eval::Evaluator;
use clap::Parser;
use std::io::{self, BufRead};

#[derive(Parser)]
#[command(name = "bbc", about = "Better BC - batteries-included calculator")]
struct Cli {
    /// Expression to evaluate (non-interactive)
    #[arg(short, long)]
    expr: Option<String>,

    /// File to evaluate
    #[arg(short, long)]
    file: Option<String>,

    /// Output base (2-36, default 10)
    #[arg(long, default_value = "10")]
    obase: u32,

    /// Decimal precision (default 20)
    #[arg(long, default_value = "20")]
    scale: u32,

    /// Additional unit sets to load (comma-separated, e.g., "imperial,scientific")
    #[arg(long, value_delimiter = ',')]
    units: Vec<String>,
}

fn main() {
    let cli = Cli::parse();

    let mut evaluator = Evaluator::new();
    let mut env = Env::new();

    // Register constants from TOML data files
    bbc_core::register_constants(&mut env);

    // Load additional unit sets
    for set_name in &cli.units {
        evaluator.registry.load_unit_set(set_name);
    }

    // Apply CLI settings
    if cli.obase != 10 {
        let _ = env.set_var("obase".into(), bbc_core::value::Value::from_int(cli.obase as i64));
    }
    if cli.scale != 20 {
        let _ = env.set_var("scale".into(), bbc_core::value::Value::from_int(cli.scale as i64));
    }

    // Register modules
    #[cfg(feature = "bitwise")]
    {
        env.register_module(Box::new(bbc_bitwise::BitwiseModule));
    }

    if let Some(expr) = cli.expr {
        // Single expression mode
        match bbc_core::evaluate_and_format(&expr, &mut env, &evaluator) {
            Ok(result) => println!("{}", result),
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    } else if let Some(file) = cli.file {
        // File mode
        let content = std::fs::read_to_string(&file).unwrap_or_else(|e| {
            eprintln!("error reading {}: {}", file, e);
            std::process::exit(1);
        });
        run_lines(&content, &mut env, &evaluator);
    } else if atty::is(atty::Stream::Stdin) {
        // Interactive REPL
        if let Err(e) = repl::run_repl(&mut env, &evaluator) {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    } else {
        // Pipe/stdin mode
        let stdin = io::stdin();
        let mut input = String::new();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => input.push_str(&format!("{}\n", l)),
                Err(e) => {
                    eprintln!("error reading stdin: {}", e);
                    std::process::exit(1);
                }
            }
        }
        run_lines(&input, &mut env, &evaluator);
    }
}

fn run_lines(input: &str, env: &mut Env, evaluator: &Evaluator) {
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match bbc_core::evaluate_and_format(line, env, evaluator) {
            Ok(result) => println!("{}", result),
            Err(e) => eprintln!("error: {}", e),
        }
    }
}
