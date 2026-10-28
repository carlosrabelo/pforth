#include "catch.hpp"
#include "forth.hpp"

TEST_CASE("Control Flow - Runtime branch execution (BRANCH, 0BRANCH)", "[control_flow]") {
    pforth::Forth f;

    int lit_xt = f.find_xt("LIT");
    int branch_xt = f.find_xt("BRANCH");
    int zbranch_xt = f.find_xt("0BRANCH");
    int exit_xt = f.find_xt("EXIT");
    int dup_xt = f.find_xt("DUP");
    int oneminus_xt = f.find_xt("1-");
    int drop_xt = f.find_xt("DROP");

    REQUIRE(lit_xt >= 0);
    REQUIRE(branch_xt >= 0);
    REQUIRE(zbranch_xt >= 0);
    REQUIRE(exit_xt >= 0);

    // Unconditional forward BRANCH jumps over dead code
    // Body: LIT 10, BRANCH 3, LIT 999, LIT 20, EXIT
    f.define_word("TESTBRANCH", pforth::WordType::Colon, nullptr, {
        static_cast<pforth::Cell>(lit_xt), 10,
        static_cast<pforth::Cell>(branch_xt), 3,
        static_cast<pforth::Cell>(lit_xt), 999,
        static_cast<pforth::Cell>(lit_xt), 20,
        static_cast<pforth::Cell>(exit_xt)
    }, false);

    f.push_ds(f.find_xt("TESTBRANCH"));
    f.word_execute();
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.ds[0] == 10);
    REQUIRE(f.ds[1] == 20);
    f.ds.clear();

    // 0BRANCH with flag == 0 jumps over branch target
    // Body: 0BRANCH 3, LIT 999, LIT 42, EXIT
    f.define_word("TESTZBRANCH_ZERO", pforth::WordType::Colon, nullptr, {
        static_cast<pforth::Cell>(zbranch_xt), 3,
        static_cast<pforth::Cell>(lit_xt), 999,
        static_cast<pforth::Cell>(lit_xt), 42,
        static_cast<pforth::Cell>(exit_xt)
    }, false);

    f.push_ds(0); // false condition
    f.push_ds(f.find_xt("TESTZBRANCH_ZERO"));
    f.word_execute();
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 42);

    // 0BRANCH with flag != 0 falls through without jumping
    f.push_ds(-1); // true condition
    f.push_ds(f.find_xt("TESTZBRANCH_ZERO"));
    f.word_execute();
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.ds[0] == 999);
    REQUIRE(f.ds[1] == 42);
    f.ds.clear();

    // Backward BRANCH loop: count down from 3 to 0
    // Body:
    // 0: LIT 3
    // 2: DUP
    // 3: 1-
    // 4: DUP
    // 5: 0BRANCH 4 (at ip=6, 6+4=10 -> DROP)
    // 7: BRANCH -6 (at ip=8, 8-6=2 -> DUP)
    // 9: 0 (padding)
    // 10: DROP
    // 11: EXIT
    f.define_word("TESTLOOP", pforth::WordType::Colon, nullptr, {
        static_cast<pforth::Cell>(lit_xt), 3,
        static_cast<pforth::Cell>(dup_xt),
        static_cast<pforth::Cell>(oneminus_xt),
        static_cast<pforth::Cell>(dup_xt),
        static_cast<pforth::Cell>(zbranch_xt), 4,
        static_cast<pforth::Cell>(branch_xt), -6,
        0,
        static_cast<pforth::Cell>(drop_xt),
        static_cast<pforth::Cell>(exit_xt)
    }, false);

    f.push_ds(f.find_xt("TESTLOOP"));
    f.word_execute();
    REQUIRE(f.ds.size() == 3);
    REQUIRE(f.ds[0] == 3);
    REQUIRE(f.ds[1] == 2);
    REQUIRE(f.ds[2] == 1);
}

TEST_CASE("Control Flow - Conditional branching compiler (IF, ELSE, THEN)", "[control_flow]") {
    pforth::Forth f;

    // IF ... THEN
    f.interpret_line(": ABSVAL DUP 0< IF NEGATE THEN ;");
    f.interpret_line("15 ABSVAL");
    REQUIRE(f.pop_ds() == 15);
    f.interpret_line("-15 ABSVAL");
    REQUIRE(f.pop_ds() == 15);

    // IF ... ELSE ... THEN
    f.interpret_line(": SIGN DUP 0> IF DROP 1 ELSE DROP -1 THEN ;");
    f.interpret_line("42 SIGN");
    REQUIRE(f.pop_ds() == 1);
    f.interpret_line("-42 SIGN");
    REQUIRE(f.pop_ds() == -1);

    // Nested IF ... ELSE ... THEN
    f.interpret_line(": CLASSIFY DUP 0= IF DROP 0 ELSE DUP 0> IF DROP 1 ELSE DROP -1 THEN THEN ;");
    f.interpret_line("0 CLASSIFY");
    REQUIRE(f.pop_ds() == 0);
    f.interpret_line("100 CLASSIFY");
    REQUIRE(f.pop_ds() == 1);
    f.interpret_line("-100 CLASSIFY");
    REQUIRE(f.pop_ds() == -1);

    // Recursive Factorial with IF / ELSE / THEN / RECURSE
    f.interpret_line(": FACT DUP 2 < IF DROP 1 ELSE DUP 1- RECURSE * THEN ;");
    f.interpret_line("0 FACT");
    REQUIRE(f.pop_ds() == 1);
    f.interpret_line("1 FACT");
    REQUIRE(f.pop_ds() == 1);
    f.interpret_line("5 FACT");
    REQUIRE(f.pop_ds() == 120);
    f.interpret_line("6 FACT");
    REQUIRE(f.pop_ds() == 720);

    // Error handling outside compilation state
    REQUIRE_THROWS_WITH(f.interpret_line("IF"), "IF ONLY IN COMPILATION");
    REQUIRE_THROWS_WITH(f.interpret_line("ELSE"), "ELSE ONLY IN COMPILATION");
    REQUIRE_THROWS_WITH(f.interpret_line("THEN"), "THEN ONLY IN COMPILATION");
}
