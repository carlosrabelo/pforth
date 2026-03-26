use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cfg = pforth::forth::parse_args(&args);
    let stdin = std::io::stdin();
    let code = pforth::forth::run(&cfg, stdin.lock(), std::io::stdout(), std::io::stderr());
    ExitCode::from(code as u8)
}
