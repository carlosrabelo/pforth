#include "catch.hpp"
#include "forth.hpp"

TEST_CASE("Dictionary - Word structure and lookup", "[dictionary]") {
    pforth::Forth f;

    REQUIRE(f.lookup("DUP") != nullptr);
    REQUIRE(f.lookup("dup") != nullptr);
    REQUIRE(f.lookup("Dup") != nullptr);
    REQUIRE(f.lookup("NONEXISTENT") == nullptr);

    int xt = f.find_xt("swap");
    REQUIRE(xt >= 0);
    REQUIRE(f.words[xt].name == "SWAP");
}

TEST_CASE("Dictionary - FIND primitive", "[dictionary]") {
    pforth::Forth f;

    // Test 1: Non-immediate word "DUP"
    // Write counted string at address 3000: len=3, 'D', 'U', 'P'
    f.c_store(3000, 3);
    f.c_store(3001, 'd');
    f.c_store(3002, 'u');
    f.c_store(3003, 'p');

    f.push_ds(3000);
    f.word_find();
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.pop_ds() == -1); // not immediate
    REQUIRE(f.pop_ds() == f.find_xt("DUP"));

    // Test 2: Immediate word
    f.define_immediate("TESTIMM", [](pforth::Forth*) {});
    f.c_store(3010, 7);
    const char* imm_name = "TESTIMM";
    for (int i = 0; i < 7; ++i) {
        f.c_store(3011 + i, imm_name[i]);
    }
    f.push_ds(3010);
    f.word_find();
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.pop_ds() == 1); // immediate flag
    REQUIRE(f.pop_ds() == f.find_xt("TESTIMM"));

    // Test 3: Unknown word
    f.c_store(3020, 4);
    f.c_store(3021, 'N');
    f.c_store(3022, 'O');
    f.c_store(3023, 'P');
    f.c_store(3024, 'E');

    f.push_ds(3020);
    f.word_find();
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.pop_ds() == 0); // not found flag
    REQUIRE(f.pop_ds() == 3020); // original string addr
}

TEST_CASE("Dictionary - Tick (') and >BODY", "[dictionary]") {
    pforth::Forth f;

    // ' with valid word in TIB
    f.tib = "  over  ";
    f.in_ptr = 0;
    f.word_tick();
    REQUIRE(f.ds.size() == 1);
    int over_xt = f.find_xt("OVER");
    REQUIRE(f.pop_ds() == over_xt);

    // ' with nonexistent word
    f.tib = "missingword";
    f.in_ptr = 0;
    REQUIRE_THROWS_WITH(f.word_tick(), "WORD NOT FOUND: missingword");

    // ' with empty input
    f.tib = "   ";
    f.in_ptr = 0;
    REQUIRE_THROWS_WITH(f.word_tick(), "WORD NAME EXPECTED");

    // >BODY with custom PFA
    pforth::Word* w = f.define_word("MYVAR", pforth::WordType::Variable, nullptr, {}, false);
    w->pfa = 5432;
    int var_xt = f.find_xt("MYVAR");

    f.push_ds(var_xt);
    f.word_to_body();
    REQUIRE(f.pop_ds() == 5432);

    // >BODY fallback (xt + 2)
    f.push_ds(100);
    f.word_to_body();
    REQUIRE(f.pop_ds() == 102);
}
