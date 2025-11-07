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

TEST_CASE("Stack Manip - ?DUP", "[stack]") {
    pforth::Forth f;

    // non-zero: duplicated
    f.push_ds(123);
    f.word_qdup();
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.pop_ds() == 123);
    REQUIRE(f.pop_ds() == 123);

    // zero: not duplicated (only 0 remains)
    f.push_ds(0);
    f.word_qdup();
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 0);

    // underflow
    REQUIRE_THROWS_WITH(f.word_qdup(), "DATA STACK UNDERFLOW");
}

TEST_CASE("Stack Manip - ROT and -ROT", "[stack]") {
    pforth::Forth f;

    // ROT ( a b c -- b c a )
    f.push_ds(10);
    f.push_ds(20);
    f.push_ds(30);
    f.word_rot();
    REQUIRE(f.ds.size() == 3);
    REQUIRE(f.pop_ds() == 10);
    REQUIRE(f.pop_ds() == 30);
    REQUIRE(f.pop_ds() == 20);

    // ROT underflow
    f.push_ds(1);
    f.push_ds(2);
    REQUIRE_THROWS_WITH(f.word_rot(), "DATA STACK UNDERFLOW");

    // -ROT ( a b c -- c a b )
    f.push_ds(10);
    f.push_ds(20);
    f.push_ds(30);
    f.word_nrot();
    REQUIRE(f.ds.size() == 3);
    REQUIRE(f.pop_ds() == 20);
    REQUIRE(f.pop_ds() == 10);
    REQUIRE(f.pop_ds() == 30);

    // -ROT underflow
    f.push_ds(1);
    f.push_ds(2);
    REQUIRE_THROWS_WITH(f.word_nrot(), "DATA STACK UNDERFLOW");
}

TEST_CASE("Stack Manip - NIP and TUCK", "[stack]") {
    pforth::Forth f;

    // NIP ( a b -- b )
    f.push_ds(11);
    f.push_ds(22);
    f.word_nip();
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 22);

    // NIP underflow
    f.push_ds(1);
    REQUIRE_THROWS_WITH(f.word_nip(), "DATA STACK UNDERFLOW");

    // TUCK ( a b -- b a b )
    f.push_ds(11);
    f.push_ds(22);
    f.word_tuck();
    REQUIRE(f.ds.size() == 3);
    REQUIRE(f.pop_ds() == 22);
    REQUIRE(f.pop_ds() == 11);
    REQUIRE(f.pop_ds() == 22);

    // TUCK underflow
    f.push_ds(1);
    REQUIRE_THROWS_WITH(f.word_tuck(), "DATA STACK UNDERFLOW");
}

