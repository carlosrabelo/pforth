#include "catch.hpp"
#include "forth.hpp"
#include <sstream>

TEST_CASE("Core Library Bootstrap - Automatic pre-loading of CORE", "[core]") {
    pforth::Forth f;
    REQUIRE(f.loaded_modules.find("CORE") != f.loaded_modules.end());

    // Words should be registered as Colon definitions (defined in Forth)
    const char* core_words[] = {
        "NIP", "TUCK", "ROT", "-ROT", "2DUP", "2DROP",
        "/", "MOD", "0=", "<>", "0<>", "0<", "0>", "<=", ">=",
        "1+", "1-", "2+", "2-", "2*", "2/", "NEGATE", "+!",
        "ABS", "MIN", "MAX", "SPACE", "SPACES", "?", "PAGE", "ERASE"
    };

    for (const char* name : core_words) {
        const pforth::Word* w = f.lookup(name);
        INFO("Checking word: " << name);
        REQUIRE(w != nullptr);
        REQUIRE(w->type == pforth::WordType::Colon);
    }
}

TEST_CASE("Core Library Bootstrap - Stack manipulation words", "[core]") {
    pforth::Forth f;

    // NIP ( a b -- b )
    f.interpret_line("10 20 NIP");
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 20);

    // TUCK ( a b -- b a b )
    f.interpret_line("10 20 TUCK");
    REQUIRE(f.ds.size() == 3);
    REQUIRE(f.pop_ds() == 20);
    REQUIRE(f.pop_ds() == 10);
    REQUIRE(f.pop_ds() == 20);

    // ROT ( a b c -- b c a )
    f.interpret_line("10 20 30 ROT");
    REQUIRE(f.ds.size() == 3);
    REQUIRE(f.pop_ds() == 10);
    REQUIRE(f.pop_ds() == 30);
    REQUIRE(f.pop_ds() == 20);

    // -ROT ( a b c -- c a b )
    f.interpret_line("10 20 30 -ROT");
    REQUIRE(f.ds.size() == 3);
    REQUIRE(f.pop_ds() == 20);
    REQUIRE(f.pop_ds() == 10);
    REQUIRE(f.pop_ds() == 30);

    // 2DUP ( a b -- a b a b )
    f.interpret_line("10 20 2DUP");
    REQUIRE(f.ds.size() == 4);
    REQUIRE(f.pop_ds() == 20);
    REQUIRE(f.pop_ds() == 10);
    REQUIRE(f.pop_ds() == 20);
    REQUIRE(f.pop_ds() == 10);

    // 2DROP ( a b -- )
    f.interpret_line("10 20 30 40 2DROP");
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.pop_ds() == 20);
    REQUIRE(f.pop_ds() == 10);
}

TEST_CASE("Core Library Bootstrap - Arithmetic and scaling words", "[core]") {
    pforth::Forth f;

    // / ( a b -- quot )
    f.interpret_line("14 3 /");
    REQUIRE(f.pop_ds() == 4);

    // MOD ( a b -- rem )
    f.interpret_line("14 3 MOD");
    REQUIRE(f.pop_ds() == 2);

    // 1+ and 1-
    f.interpret_line("10 1+ 10 1-");
    REQUIRE(f.pop_ds() == 9);
    REQUIRE(f.pop_ds() == 11);

    // 2+ and 2-
    f.interpret_line("10 2+ 10 2-");
    REQUIRE(f.pop_ds() == 8);
    REQUIRE(f.pop_ds() == 12);

    // 2* and 2/
    f.interpret_line("10 2* 10 2/");
    REQUIRE(f.pop_ds() == 5);
    REQUIRE(f.pop_ds() == 20);

    // NEGATE
    f.interpret_line("42 NEGATE -50 NEGATE");
    REQUIRE(f.pop_ds() == 50);
    REQUIRE(f.pop_ds() == -42);

    // +! ( n addr -- )
    f.interpret_line("VARIABLE MYACC 100 MYACC ! 25 MYACC +! MYACC @");
    REQUIRE(f.pop_ds() == 125);

    // ABS
    f.interpret_line("-123 ABS 456 ABS");
    REQUIRE(f.pop_ds() == 456);
    REQUIRE(f.pop_ds() == 123);

    // MIN and MAX
    f.interpret_line("10 20 MIN 20 10 MIN");
    REQUIRE(f.pop_ds() == 10);
    REQUIRE(f.pop_ds() == 10);

    f.interpret_line("10 20 MAX 20 10 MAX");
    REQUIRE(f.pop_ds() == 20);
    REQUIRE(f.pop_ds() == 20);
}

TEST_CASE("Core Library Bootstrap - Comparison words", "[core]") {
    pforth::Forth f;

    // 0=
    f.interpret_line("0 0= 5 0=");
    REQUIRE(f.pop_ds() == 0);
    REQUIRE(f.pop_ds() == -1);

    // <>
    f.interpret_line("5 5 <> 5 6 <>");
    REQUIRE(f.pop_ds() == -1);
    REQUIRE(f.pop_ds() == 0);

    // 0<>
    f.interpret_line("0 0<> 5 0<>");
    REQUIRE(f.pop_ds() == -1);
    REQUIRE(f.pop_ds() == 0);

    // 0< and 0>
    f.interpret_line("-5 0< 0 0< 5 0<");
    REQUIRE(f.pop_ds() == 0);
    REQUIRE(f.pop_ds() == 0);
    REQUIRE(f.pop_ds() == -1);

    f.interpret_line("5 0> 0 0> -5 0>");
    REQUIRE(f.pop_ds() == 0);
    REQUIRE(f.pop_ds() == 0);
    REQUIRE(f.pop_ds() == -1);

    // <=
    f.interpret_line("2 3 <= 3 3 <= 4 3 <=");
    REQUIRE(f.pop_ds() == 0);
    REQUIRE(f.pop_ds() == -1);
    REQUIRE(f.pop_ds() == -1);

    // >=
    f.interpret_line("4 3 >= 3 3 >= 2 3 >=");
    REQUIRE(f.pop_ds() == 0);
    REQUIRE(f.pop_ds() == -1);
    REQUIRE(f.pop_ds() == -1);
}

TEST_CASE("Core Library Bootstrap - Formatting and memory words", "[core]") {
    std::ostringstream out;
    pforth::Forth f(nullptr, &out);

    // SPACE and SPACES
    f.interpret_line("SPACE");
    REQUIRE(out.str() == " ");
    out.str("");

    f.interpret_line("3 SPACES");
    REQUIRE(out.str() == "   ");
    out.str("");

    // ? prints variable value
    f.interpret_line("VARIABLE QVAR 42 QVAR ! QVAR ?");
    REQUIRE(out.str() == "42 ");
    out.str("");

    // PAGE
    f.interpret_line("PAGE");
    REQUIRE((out.str() == "\014" || out.str() == "\033[2J\033[H"));
    out.str("");

    // ERASE ( addr len -- )
    f.interpret_line("CREATE MYBUF 8 ALLOT");
    f.interpret_line("MYBUF 8 255 FILL");
    f.interpret_line("MYBUF");
    pforth::Cell buf_addr = f.pop_ds();
    REQUIRE(f.c_fetch(buf_addr) == 255);
    f.interpret_line("MYBUF 8 ERASE");
    REQUIRE(f.c_fetch(buf_addr) == 0);
    REQUIRE(f.c_fetch(buf_addr + 7) == 0);
}
