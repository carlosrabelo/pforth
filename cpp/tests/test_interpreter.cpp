#include "catch.hpp"
#include "forth.hpp"

TEST_CASE("Interpreter - BASE variable", "[interpreter]") {
    pforth::Forth f;

    // BASE pushes 1004
    f.word_base();
    REQUIRE(f.pop_ds() == 1004);
    REQUIRE(f.get_base() == 10);

    // Change BASE to 16
    f.push_ds(16);
    f.push_ds(1004);
    f.word_store();
    REQUIRE(f.get_base() == 16);
}

TEST_CASE("Interpreter - Number parsing", "[interpreter]") {
    pforth::Forth f;
    pforth::Cell val = 0;

    // Decimal
    REQUIRE(f.parse_number("123", val));
    REQUIRE(val == 123);

    REQUIRE(f.parse_number("-456", val));
    REQUIRE(val == -456);

    REQUIRE(f.parse_number("+789", val));
    REQUIRE(val == 789);

    // Hex prefix $
    REQUIRE(f.parse_number("$1F", val));
    REQUIRE(val == 31);

    REQUIRE(f.parse_number("-$10", val));
    REQUIRE(val == -16);

    // Binary prefix %
    REQUIRE(f.parse_number("%1010", val));
    REQUIRE(val == 10);

    // Current base = 16
    f.set_base(16);
    REQUIRE(f.parse_number("FF", val));
    REQUIRE(val == 255);

    // Current base = 8
    f.set_base(8);
    REQUIRE(f.parse_number("77", val));
    REQUIRE(val == 63);

    // Invalid numbers
    f.set_base(10);
    REQUIRE_FALSE(f.parse_number("ABC", val));
    REQUIRE_FALSE(f.parse_number("", val));
    REQUIRE_FALSE(f.parse_number("-", val));
}

TEST_CASE("Interpreter - WORD and NUMBER primitives", "[interpreter]") {
    pforth::Forth f;

    // WORD ( delim -- addr )
    f.tib = "   HELLO   WORLD";
    f.in_ptr = 0;
    f.push_ds(' ');
    f.word_word();
    REQUIRE(f.ds.size() == 1);
    pforth::Cell addr = f.pop_ds();
    REQUIRE(addr == f.here());
    REQUIRE(f.c_fetch(addr) == 5);
    std::string word_read;
    for (int i = 0; i < 5; ++i) {
        word_read += static_cast<char>(f.c_fetch(addr + 1 + i));
    }
    REQUIRE(word_read == "HELLO");

    // NUMBER ( addr -- n flag )
    // Write counted string at address 4000: len 4, "1234"
    f.c_store(4000, 4);
    f.c_store(4001, '1');
    f.c_store(4002, '2');
    f.c_store(4003, '3');
    f.c_store(4004, '4');

    f.push_ds(4000);
    f.word_number();
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.pop_ds() == -1); // success
    REQUIRE(f.pop_ds() == 1234);

    // Non-number: "XYZ"
    f.c_store(4010, 3);
    f.c_store(4011, 'X');
    f.c_store(4012, 'Y');
    f.c_store(4013, 'Z');

    f.push_ds(4010);
    f.word_number();
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.pop_ds() == 0); // failure
    REQUIRE(f.pop_ds() == 0);
}

TEST_CASE("Interpreter - Outer interpreter execution (INTERPRET, ABORT)", "[interpreter]") {
    pforth::Forth f;

    // Sequence of numbers and arithmetic
    f.interpret_line("10 20 +");
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 30);

    // DUP and multiply
    f.interpret_line("5 DUP *");
    REQUIRE(f.pop_ds() == 25);

    // Multi-word line
    f.interpret_line("1 2 3 4 + + +");
    REQUIRE(f.pop_ds() == 10);

    // Error handling on unknown word
    f.push_ds(100);
    REQUIRE_THROWS_WITH(f.interpret_line("UNKNOWNWORD 50 +"), "UNKNOWN WORD: UNKNOWNWORD");
    // ABORT cleared stack and line
    REQUIRE(f.ds.empty());
    REQUIRE(f.state == false);
}

TEST_CASE("Interpreter - Compiler state and directives (STATE, [, ], LIT, LITERAL)", "[interpreter]") {
    pforth::Forth f;

    // Initial STATE is 0
    f.word_state();
    REQUIRE(f.pop_ds() == 0);

    // ] sets STATE to true (-1)
    f.word_right_bracket();
    f.word_state();
    REQUIRE(f.pop_ds() == -1);
    REQUIRE(f.state == true);

    // [ sets STATE to false (0)
    f.word_left_bracket();
    f.word_state();
    REQUIRE(f.pop_ds() == 0);
    REQUIRE(f.state == false);

    // LIT runtime execution
    int lit_xt = f.find_xt("LIT");
    pforth::Word* w_lit_test = f.define_word("TESTLIT", pforth::WordType::Colon, nullptr, {static_cast<pforth::Cell>(lit_xt), 555}, false);
    (void)w_lit_test;
    int test_xt = f.find_xt("TESTLIT");
    f.push_ds(test_xt);
    f.word_execute();
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 555);

    // LITERAL compiles LIT followed by value
    f.compile_list.clear();
    f.push_ds(888);
    f.word_literal();
    REQUIRE(f.compile_list.size() == 2);
    REQUIRE(f.compile_list[0] == lit_xt);
    REQUIRE(f.compile_list[1] == 888);

    // COMPILE compiles next word's XT
    f.compile_list.clear();
    f.tib = "SWAP";
    f.in_ptr = 0;
    f.word_compile();
    REQUIRE(f.compile_list.size() == 1);
    REQUIRE(f.compile_list[0] == f.find_xt("SWAP"));

    // [COMPILE] forces compilation of immediate word
    f.compile_list.clear();
    f.tib = "[";
    f.in_ptr = 0;
    f.word_bracket_compile();
    REQUIRE(f.compile_list.size() == 1);
    REQUIRE(f.compile_list[0] == f.find_xt("["));
}


