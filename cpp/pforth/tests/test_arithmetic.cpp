#include "catch.hpp"
#include "forth.hpp"

TEST_CASE("Arithmetic - Basic +, -, * operations", "[arithmetic]") {
    pforth::Forth f;

    // + ( a b -- sum )
    f.push_ds(15);
    f.push_ds(27);
    f.word_plus();
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 42);

    // Negative addition
    f.push_ds(10);
    f.push_ds(-25);
    f.word_plus();
    REQUIRE(f.pop_ds() == -15);

    // - ( a b -- diff ) 50 - 20 = 30
    f.push_ds(50);
    f.push_ds(20);
    f.word_minus();
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 30);

    // * ( a b -- prod )
    f.push_ds(6);
    f.push_ds(7);
    f.word_multiply();
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 42);

    f.push_ds(-4);
    f.push_ds(5);
    f.word_multiply();
    REQUIRE(f.pop_ds() == -20);
}

TEST_CASE("Arithmetic - Division /MOD, /, and MOD", "[arithmetic]") {
    pforth::Forth f;

    // /MOD ( a b -- rem quot ) 17 / 5 = 3 rem 2
    f.push_ds(17);
    f.push_ds(5);
    f.word_slash_mod();
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.pop_ds() == 3); // quot on top
    REQUIRE(f.pop_ds() == 2); // rem next

    // / ( a b -- quot ) 42 / 6 = 7
    f.push_ds(42);
    f.push_ds(6);
    f.word_slash();
    REQUIRE(f.pop_ds() == 7);

    // MOD ( a b -- rem ) 42 % 5 = 2
    f.push_ds(42);
    f.push_ds(5);
    f.word_mod();
    REQUIRE(f.pop_ds() == 2);

    // Division by zero throws DIVISION BY ZERO
    f.push_ds(10);
    f.push_ds(0);
    REQUIRE_THROWS_WITH(f.word_slash_mod(), "DIVISION BY ZERO");

    f.push_ds(10);
    f.push_ds(0);
    REQUIRE_THROWS_WITH(f.word_slash(), "DIVISION BY ZERO");

    f.push_ds(10);
    f.push_ds(0);
    REQUIRE_THROWS_WITH(f.word_mod(), "DIVISION BY ZERO");
}

TEST_CASE("Arithmetic - NEGATE", "[arithmetic]") {
    pforth::Forth f;

    f.push_ds(42);
    f.word_negate();
    REQUIRE(f.pop_ds() == -42);

    f.push_ds(-100);
    f.word_negate();
    REQUIRE(f.pop_ds() == 100);

    f.push_ds(0);
    f.word_negate();
    REQUIRE(f.pop_ds() == 0);

    REQUIRE_THROWS_WITH(f.word_negate(), "DATA STACK UNDERFLOW");
}

TEST_CASE("Arithmetic - Scaling and increments (1+, 1-, 2+, 2-, 2*, 2/)", "[arithmetic]") {
    pforth::Forth f;

    // 1+ and 1-
    f.push_ds(10);
    f.word_one_plus();
    REQUIRE(f.pop_ds() == 11);

    f.push_ds(10);
    f.word_one_minus();
    REQUIRE(f.pop_ds() == 9);

    // 2+ and 2-
    f.push_ds(20);
    f.word_two_plus();
    REQUIRE(f.pop_ds() == 22);

    f.push_ds(20);
    f.word_two_minus();
    REQUIRE(f.pop_ds() == 18);

    // 2* (left shift)
    f.push_ds(7);
    f.word_two_star();
    REQUIRE(f.pop_ds() == 14);

    f.push_ds(-5);
    f.word_two_star();
    REQUIRE(f.pop_ds() == -10);

    // 2/ (arithmetic right shift)
    f.push_ds(14);
    f.word_two_slash();
    REQUIRE(f.pop_ds() == 7);

    f.push_ds(-10);
    f.word_two_slash();
    REQUIRE(f.pop_ds() == -5);
}

TEST_CASE("Arithmetic - ABS, MIN, and MAX", "[arithmetic]") {
    pforth::Forth f;

    // ABS
    f.push_ds(-42);
    f.word_abs();
    REQUIRE(f.pop_ds() == 42);

    f.push_ds(100);
    f.word_abs();
    REQUIRE(f.pop_ds() == 100);

    // MIN
    f.push_ds(10);
    f.push_ds(20);
    f.word_min();
    REQUIRE(f.pop_ds() == 10);

    f.push_ds(-15);
    f.push_ds(-5);
    f.word_min();
    REQUIRE(f.pop_ds() == -15);

    // MAX
    f.push_ds(10);
    f.push_ds(20);
    f.word_max();
    REQUIRE(f.pop_ds() == 20);

    f.push_ds(-15);
    f.push_ds(-5);
    f.word_max();
    REQUIRE(f.pop_ds() == -5);
}

TEST_CASE("Arithmetic - Bitwise logic (AND, OR, XOR, INVERT)", "[arithmetic]") {
    pforth::Forth f;

    // AND
    f.push_ds(0x00FF);
    f.push_ds(0x0F0F);
    f.word_and();
    REQUIRE(f.pop_ds() == 0x000F);

    // OR
    f.push_ds(0x00F0);
    f.push_ds(0x0F00);
    f.word_or();
    REQUIRE(f.pop_ds() == 0x0FF0);

    // XOR
    f.push_ds(static_cast<pforth::Cell>(0xAAAA));
    f.push_ds(0x5555);
    f.word_xor();
    REQUIRE(f.pop_ds() == -1); // 0xFFFF

    // INVERT
    f.push_ds(0);
    f.word_invert();
    REQUIRE(f.pop_ds() == -1);

    f.push_ds(-1);
    f.word_invert();
    REQUIRE(f.pop_ds() == 0);

    // Underflow
    REQUIRE_THROWS_WITH(f.word_invert(), "DATA STACK UNDERFLOW");
    f.push_ds(1);
    REQUIRE_THROWS_WITH(f.word_and(), "DATA STACK UNDERFLOW");
}


