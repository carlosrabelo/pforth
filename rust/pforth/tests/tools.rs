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
