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

TEST_CASE("Dictionary - Memory allocation (DP, HERE, PAD, ALLOT, comma, c-comma)", "[dictionary]") {
    pforth::Forth f;

    // Initial DP and HERE
    f.word_dp();
    REQUIRE(f.pop_ds() == 1002);

    f.word_here();
    REQUIRE(f.pop_ds() == 1024);

    f.word_pad();
    REQUIRE(f.pop_ds() == 1024 + 68);

    // ALLOT 10 bytes
    f.push_ds(10);
    f.word_allot();
    f.word_here();
    REQUIRE(f.pop_ds() == 1034);

    // Comma (,) writes cell and advances 2 bytes
    f.push_ds(0x1234);
    f.word_comma();
    f.word_here();
    REQUIRE(f.pop_ds() == 1036);
    REQUIRE(f.fetch_cell(1034) == 0x1234);

    // C-comma (C,) writes byte and advances 1 byte
    f.push_ds(0xAB);
    f.word_c_comma();
    f.word_here();
    REQUIRE(f.pop_ds() == 1037);
    REQUIRE(f.c_fetch(1036) == 0xAB);
}

TEST_CASE("Dictionary - Execution token dispatcher (EXECUTE)", "[dictionary]") {
    pforth::Forth f;

    // 1. Primitive execution: '+'
    int plus_xt = f.find_xt("+");
    f.push_ds(12);
    f.push_ds(18);
    f.push_ds(plus_xt);
    f.word_execute();
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 30);

    // 2. Constant execution: pushes body[0]
    f.define_word("MYCONST", pforth::WordType::Constant, nullptr, {999}, false);
    int const_xt = f.find_xt("MYCONST");
    f.push_ds(const_xt);
    f.word_execute();
    REQUIRE(f.pop_ds() == 999);

    // 3. Variable execution: pushes PFA
    pforth::Word* vword = f.define_word("MYVAR", pforth::WordType::Variable, nullptr, {}, false);
    vword->pfa = 2048;
    int var_xt = f.find_xt("MYVAR");
    f.push_ds(var_xt);
    f.word_execute();
    REQUIRE(f.pop_ds() == 2048);

    // 4. Create execution: pushes PFA
    pforth::Word* crword = f.define_word("MYBUF", pforth::WordType::Create, nullptr, {}, false);
    crword->pfa = 4096;
    int cr_xt = f.find_xt("MYBUF");
    f.push_ds(cr_xt);
    f.word_execute();
    REQUIRE(f.pop_ds() == 4096);

    // 5. Invalid XT
    f.push_ds(9999);
    REQUIRE_THROWS_WITH(f.word_execute(), "INVALID EXECUTION TOKEN: 9999");
}

TEST_CASE("Dictionary - Defining words (CREATE, CONSTANT, VARIABLE)", "[dictionary]") {
    pforth::Forth f;

    // 1. CREATE
    pforth::Cell before_create = f.here();
    f.tib = "MYBUF";
    f.in_ptr = 0;
    f.word_create();

    pforth::Word* w_buf = f.lookup("MYBUF");
    REQUIRE(w_buf != nullptr);
    REQUIRE(w_buf->type == pforth::WordType::Create);
    REQUIRE(w_buf->pfa == before_create);

    // Executing MYBUF pushes its PFA
    int buf_xt = f.find_xt("MYBUF");
    f.push_ds(buf_xt);
    f.word_execute();
    REQUIRE(f.pop_ds() == before_create);

    // 2. VARIABLE
    pforth::Cell before_var = f.here();
    f.tib = "COUNTER";
    f.in_ptr = 0;
    f.word_variable();

    pforth::Word* w_var = f.lookup("COUNTER");
    REQUIRE(w_var != nullptr);
    REQUIRE(w_var->type == pforth::WordType::Variable);
    REQUIRE(w_var->pfa == before_var);
    REQUIRE(f.here() == before_var + 2); // 2 bytes allocated

    // Value initially 0
    REQUIRE(f.fetch_cell(before_var) == 0);

    // Executing COUNTER pushes its PFA
    int var_xt = f.find_xt("COUNTER");
    f.push_ds(var_xt);
    f.word_execute();
    REQUIRE(f.peek_ds() == before_var);

    // Fetch value using @: ( addr -- 0 )
    f.word_fetch();
    REQUIRE(f.pop_ds() == 0);

    // Store new value 77 and fetch back
    f.push_ds(77);
    f.push_ds(var_xt);
    f.word_execute(); // pushes addr
    f.word_store();   // store 77
    REQUIRE(f.fetch_cell(before_var) == 77);

    // 3. CONSTANT
    f.push_ds(9876);
    f.tib = "LIMIT";
    f.in_ptr = 0;
    f.word_constant();

    pforth::Word* w_const = f.lookup("LIMIT");
    REQUIRE(w_const != nullptr);
    REQUIRE(w_const->type == pforth::WordType::Constant);

    int const_xt = f.find_xt("LIMIT");
    f.push_ds(const_xt);
    f.word_execute();
    REQUIRE(f.pop_ds() == 9876);

    // Missing word name errors
    f.tib = "   ";
    f.in_ptr = 0;
    REQUIRE_THROWS_WITH(f.word_create(), "WORD NAME EXPECTED");

    f.tib = "   ";
    f.in_ptr = 0;
    REQUIRE_THROWS_WITH(f.word_variable(), "WORD NAME EXPECTED");

    f.push_ds(1);
    f.tib = "   ";
    f.in_ptr = 0;
    REQUIRE_THROWS_WITH(f.word_constant(), "WORD NAME EXPECTED");
}



