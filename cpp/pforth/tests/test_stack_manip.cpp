#include "catch.hpp"
#include "forth.hpp"

TEST_CASE("Stack Manip - Basic DUP and DROP", "[stack]") {
    pforth::Forth f;

    // DUP
    f.push_ds(42);
    f.word_dup();
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.pop_ds() == 42);
    REQUIRE(f.pop_ds() == 42);

    // DUP underflow
    REQUIRE_THROWS_WITH(f.word_dup(), "DATA STACK UNDERFLOW");

    // DROP
    f.push_ds(10);
    f.push_ds(20);
    f.word_drop();
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 10);

    // DROP underflow
    REQUIRE_THROWS_WITH(f.word_drop(), "DATA STACK UNDERFLOW");
}

TEST_CASE("Stack Manip - Basic SWAP and OVER", "[stack]") {
    pforth::Forth f;

    // SWAP ( a b -- b a )
    f.push_ds(1);
    f.push_ds(2);
    f.word_swap();
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.pop_ds() == 1);
    REQUIRE(f.pop_ds() == 2);

    // SWAP underflow
    f.push_ds(1);
    REQUIRE_THROWS_WITH(f.word_swap(), "DATA STACK UNDERFLOW");

    // OVER ( a b -- a b a )
    f.push_ds(100);
    f.push_ds(200);
    f.word_over();
    REQUIRE(f.ds.size() == 3);
    REQUIRE(f.pop_ds() == 100);
    REQUIRE(f.pop_ds() == 200);
    REQUIRE(f.pop_ds() == 100);

    // OVER underflow
    f.push_ds(1);
    REQUIRE_THROWS_WITH(f.word_over(), "DATA STACK UNDERFLOW");
}
