use pforth::forth::{parse_args, run, Config};
use std::io::Cursor;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn temp_script(tag: &str, text: &str) -> String {
    let path = std::env::temp_dir().join(format!("pforth_cli_{}_{}.fs", std::process::id(), tag));
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_string()
}

#[test]
fn no_arguments_select_interactive_mode() {
    let cfg = parse_args(&args(&["pforth"]));
    assert!(cfg.interactive);
    assert!(cfg.file_paths.is_empty());
}

#[test]
fn file_arguments_keep_their_order() {
    let cfg = parse_args(&args(&["pforth", "libs/core.fs", "demos/sieve.fs"]));
    assert!(!cfg.interactive);
    assert_eq!(cfg.file_paths, vec!["libs/core.fs", "demos/sieve.fs"]);
}

#[test]
fn interactive_run_starts_the_repl() {
    let cfg = parse_args(&args(&["pforth"]));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(&cfg, Cursor::new(b"6 7 * .\n".to_vec()), &mut out, &mut err);
    assert_eq!(code, 0);
    assert_eq!(String::from_utf8(out).unwrap(), " ok\n42  ok\n");
    assert!(err.is_empty());
}

#[test]
fn files_load_in_order_without_the_repl() {
    let first = temp_script("first", ": TWICE DUP + ;\n1 .\n");
    let second = temp_script("second", "4 TWICE .\n");
    let cfg = Config {
        interactive: false,
        file_paths: vec![first.clone(), second.clone()],
    };
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(&cfg, Cursor::new(Vec::new()), &mut out, &mut err);
    std::fs::remove_file(first).unwrap();
    std::fs::remove_file(second).unwrap();
    assert_eq!(code, 0);
    assert_eq!(String::from_utf8(out).unwrap(), "1 8 ");
    assert!(err.is_empty());
}

#[test]
fn missing_file_reports_cannot_open_and_returns_one() {
    let later = temp_script("later", "99 .\n");
    let missing = "non_existent_file_xyz_123.fs".to_string();
    let cfg = Config {
        interactive: false,
        file_paths: vec![missing.clone(), later.clone()],
    };
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(&cfg, Cursor::new(Vec::new()), &mut out, &mut err);
    std::fs::remove_file(later).unwrap();
    assert_eq!(code, 1);
    assert!(out.is_empty());
    assert_eq!(
        String::from_utf8(err).unwrap(),
        format!("CANNOT OPEN: {missing}\n")
    );
}
