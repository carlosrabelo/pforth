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
