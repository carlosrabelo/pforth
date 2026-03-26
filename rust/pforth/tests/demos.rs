use pforth::forth::Forth;
use std::io::Cursor;
use std::path::PathBuf;

fn find_demo(name: &str) -> PathBuf {
    if let Ok(dir) = std::env::var("PFORTH_DEMOS") {
        let p = PathBuf::from(dir).join(name);
        if p.is_file() {
            return p;
        }
    }
    let mut dir = std::env::current_dir().expect("cwd");
    loop {
        let candidate = dir.join("demos").join(name);
        if candidate.is_file() {
            return candidate;
        }
        if !dir.pop() {
            panic!("demo not found: {name}");
        }
    }
}

fn run_demo_file(name: &str) -> String {
    let path = find_demo(name);
    let mut out = Vec::new();
    {
        let mut forth = Forth::new(Cursor::new(Vec::new()), &mut out);
        forth.load_file(path.to_str().unwrap()).unwrap();
    }
    String::from_utf8(out).unwrap()
}

#[test]
fn hello_world() {
    assert_eq!(run_demo_file("hello.fs"), "Hello, world!\n");
}

#[test]
fn factorial() {
    assert_eq!(run_demo_file("factorial.fs"), "120 \n5040 \n");
}

#[test]
fn fibonacci() {
    assert_eq!(run_demo_file("fibonacci.fs"), "55 \n");
}

#[test]
fn bubble_sort() {
    assert_eq!(run_demo_file("bubble_sort.fs"), "0 1 2 3 4 5 6 7 8 9 \n");
}

#[test]
fn sieve_of_eratosthenes() {
    assert_eq!(run_demo_file("sieve.fs"), "1899 \n");
}

#[test]
fn tower_of_hanoi() {
    assert_eq!(
        run_demo_file("hanoi.fs"),
        "Move 1 -> 3 \nMove 1 -> 2 \nMove 3 -> 2 \nMove 1 -> 3 \nMove 2 -> 1 \nMove 2 -> 3 \nMove 1 -> 3 \n"
    );
}
