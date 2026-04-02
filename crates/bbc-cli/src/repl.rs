use bbc_core::env::Env;
use bbc_core::eval::Evaluator;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

pub fn run_repl(env: &mut Env, evaluator: &mut Evaluator) -> Result<(), Box<dyn std::error::Error>> {
    let mut rl = DefaultEditor::new()?;

    let history_path = dirs_home().map(|h| h.join(".bbc_history"));
    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    loop {
        match rl.readline("> ") {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if line == "quit" || line == "exit" {
                    break;
                }

                let _ = rl.add_history_entry(line);

                match bbc_core::evaluate_and_format(line, env, evaluator) {
                    Ok(result) => println!("{}", result),
                    Err(e) => eprintln!("error: {}", e),
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C: cancel current line
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D: exit
                break;
            }
            Err(e) => {
                eprintln!("error: {}", e);
                break;
            }
        }
    }

    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }

    Ok(())
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}
