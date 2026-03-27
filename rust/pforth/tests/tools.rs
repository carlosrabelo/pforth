use pforth::forth::Forth;
use std::io::Cursor;

type Engine = Forth<Cursor<Vec<u8>>, Vec<u8>>;

fn engine() -> Engine {
    Forth::new(Cursor::new(Vec::new()), Vec::new())
}

fn take_output(forth: &mut Engine) -> String {
    String::from_utf8(std::mem::take(&mut forth.output)).unwrap()
}

#[test]
fn words_and_dump() {
    let mut f = engine();
    take_output(&mut f);

    f.interpret_line(": AAA 1 ;").unwrap();
    f.interpret_line(": BBB 2 ; IMMEDIATE").unwrap();
    f.interpret_line("WORDS").unwrap();
    let listing = take_output(&mut f);
    assert!(listing.starts_with("I:BBB AAA "));
    assert!(listing.ends_with(" DUP\n"));
    assert_eq!(listing.matches('\n').count(), 1);

    for i in 0..16 {
        f.memory[0x3000 + i] = 0x41 + i as u8;
    }
    f.interpret_line("12288 16 DUMP").unwrap();
    assert_eq!(
        take_output(&mut f),
        "3000  41 42 43 44 45 46 47 48 49 4A 4B 4C 4D 4E 4F 50  ABCDEFGHIJKLMNOP\n"
    );

    f.memory[0x3010] = 0x00;
    f.memory[0x3011] = 0x7F;
    f.memory[0x3012] = 0x20;
    f.memory[0x3013] = 0x7E;
    f.interpret_line("12288 20 DUMP").unwrap();
    assert_eq!(
        take_output(&mut f),
        "3000  41 42 43 44 45 46 47 48 49 4A 4B 4C 4D 4E 4F 50  ABCDEFGHIJKLMNOP\n\
         3010  00 7F 20 7E  .. ~\n"
    );

    f.interpret_line("0 0 DUMP").unwrap();
    f.interpret_line("100 -1 DUMP").unwrap();
    assert!(take_output(&mut f).is_empty());

    assert_eq!(
        f.interpret_line("65530 10 DUMP").unwrap_err().message,
        "OUT OF MEMORY RANGE"
    );
    assert_eq!(take_output(&mut f), "FFFA  00 00 00 00 00 00  ......\n");

    assert_eq!(
        f.interpret_line("DUMP").unwrap_err().message,
        "DATA STACK UNDERFLOW"
    );
}

#[test]
fn running_and_eof() {
    let mut f = Forth::new(Cursor::new(Vec::new()), Vec::new());
    assert!(f.running);
    f.running_word().unwrap();
    assert_eq!(f.ds.pop(), Some(-1));
    f.push_ds(2000);
    f.push_ds(20);
    f.expect().unwrap();
    assert!(!f.running);
    f.running_word().unwrap();
    assert_eq!(f.ds.pop(), Some(0));
}

#[test]
fn load_file_and_save_file() {
    let dir = std::env::temp_dir().join(format!("pforth_lsf_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("data.txt");
    std::fs::write(&path, b"hello").unwrap();
    let name = path.to_str().unwrap();
    let missing = dir.join("missing.txt");
    let missing = missing.to_str().unwrap();

    let mut f = engine();
    assert_eq!(
        f.interpret_line("0 0 SAVE-FILE").unwrap_err().message,
        "EDITOR FILE"
    );
    assert_eq!(
        f.interpret_line("2000 100 LOAD-FILE").unwrap_err().message,
        "FILE NAME EXPECTED"
    );
    f.interpret_line(&format!("2000 100 LOAD-FILE {name}"))
        .unwrap();
    assert!(f.ds.pop().unwrap() >= 5);
    assert_eq!(f.memory[2000], b'h');
    assert_eq!(f.memory[2004], b'o');
    f.interpret_line(&format!("2000 3 LOAD-FILE {name}"))
        .unwrap();
    assert_eq!(f.ds.pop(), Some(3));

    f.memory[3000] = b'o';
    f.memory[3001] = b'k';
    f.interpret_line("3000 2 SAVE-FILE").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"ok");
    f.interpret_line("3000 0 SAVE-FILE").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"");

    assert_eq!(
        f.interpret_line(&format!("2000 10 LOAD-FILE {missing}"))
            .unwrap_err()
            .message,
        format!("CANNOT OPEN: {missing}")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rest_of_line_and_error() {
    let mut f = engine();
    f.tib = "  hello world".to_string();
    f.in_ptr = 0;
    f.push_ds(4000);
    f.push_ds(64);
    f.rest_of_line().unwrap();
    assert_eq!(f.ds.pop(), Some(11));
    assert_eq!(f.memory[4000], b'h');
    assert_eq!(f.memory[4010], b'd');
    assert_eq!(f.in_ptr, f.tib.len());

    for (i, b) in b"BOOM!".iter().enumerate() {
        f.memory[5000 + i] = *b;
    }
    f.push_ds(5000);
    f.push_ds(5);
    assert_eq!(f.error_word().unwrap_err().message, "BOOM!");
}

#[test]
fn block_editor() {
    let mut f = engine();
    take_output(&mut f);

    assert_eq!(
        f.interpret_line("LIST").unwrap_err().message,
        "UNKNOWN WORD: LIST"
    );
    f.interpret_line("#REQUIRE EDITOR").unwrap();

    assert_eq!(
        f.interpret_line("FLUSH").unwrap_err().message,
        "EDITOR FILE"
    );
    assert_eq!(
        f.interpret_line("OPEN").unwrap_err().message,
        "FILE NAME EXPECTED"
    );
    assert_eq!(
        f.interpret_line("OPEN /no/such/dir/block.txt")
            .unwrap_err()
            .message,
        "CANNOT OPEN: /no/such/dir/block.txt"
    );

    let path = std::env::temp_dir().join(format!("pforth_editor_{}.blk", std::process::id()));
    std::fs::write(&path, "first line").unwrap();
    let name = path.to_str().unwrap();
    f.interpret_line(&format!("OPEN {name}")).unwrap();
    f.interpret_line("LIST").unwrap();
    let mut expected = String::from("0: first line\n");
    for line in 1..16 {
        expected.push_str(&format!("{line}: \n"));
    }
    assert_eq!(take_output(&mut f), expected);

    f.interpret_line("0 P hello world").unwrap();
    f.interpret_line("15 P last").unwrap();
    f.interpret_line("3 P").unwrap();
    assert_eq!(
        f.interpret_line("16 P x").unwrap_err().message,
        "EDITOR LINE"
    );
    assert_eq!(
        f.interpret_line("-1 P x").unwrap_err().message,
        "EDITOR LINE"
    );
    f.interpret_line("LIST").unwrap();
    let listing = take_output(&mut f);
    assert!(listing.starts_with("0: hello world\n1: \n2: \n3: \n"));
    assert!(listing.ends_with("15: last\n"));

    let long = "x".repeat(70);
    f.interpret_line(&format!("1 P {long}")).unwrap();
    f.interpret_line("FLUSH").unwrap();
    let saved = std::fs::read(&path).unwrap();
    assert_eq!(saved.len(), 1024);
    assert_eq!(
        &saved[..64],
        format!("hello world{}", " ".repeat(53)).as_bytes()
    );
    assert_eq!(&saved[64..128], "x".repeat(64).as_bytes());
    assert_eq!(&saved[128..132], b"    ");
    assert_eq!(&saved[15 * 64..15 * 64 + 4], b"last");

    f.interpret_line(&format!("OPEN {name}")).unwrap();
    f.interpret_line("LIST").unwrap();
    assert!(take_output(&mut f).starts_with("0: hello world\n1: xxxx"));
    std::fs::remove_file(&path).unwrap();
}
