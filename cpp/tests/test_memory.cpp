#include "catch.hpp"
#include "forth.hpp"

TEST_CASE("Memory - Byte store and fetch with bounds check", "[memory]") {
    pforth::Forth f;

    f.c_store(0, 0xAB);
    REQUIRE(f.c_fetch(0) == 0xAB);

    f.c_store(65535, 0xFE);
    REQUIRE(f.c_fetch(65535) == 0xFE);

    // Out of bounds
    REQUIRE_THROWS_WITH(f.c_fetch(65536), "OUT OF MEMORY RANGE");
    REQUIRE_THROWS_WITH(f.c_store(65536, 1), "OUT OF MEMORY RANGE");
}

TEST_CASE("Memory - 16-bit cell little-endian encoding and decoding", "[memory]") {
    pforth::Forth f;

    // 0x1234 -> low byte 0x34, high byte 0x12
    pforth::Cell val = 0x1234;
    f.store_cell(1024, val);

    REQUIRE(f.c_fetch(1024) == 0x34);
    REQUIRE(f.c_fetch(1025) == 0x12);
    REQUIRE(f.fetch_cell(1024) == val);

    // Negative cell value (e.g. -1 -> 0xFFFF)
    f.store_cell(2048, -1);
    REQUIRE(f.c_fetch(2048) == 0xFF);
    REQUIRE(f.c_fetch(2049) == 0xFF);
    REQUIRE(f.fetch_cell(2048) == -1);

    // Cell out of bounds
    REQUIRE_THROWS_WITH(f.fetch_cell(65535), "OUT OF MEMORY RANGE");
    REQUIRE_THROWS_WITH(f.store_cell(65535, 10), "OUT OF MEMORY RANGE");
}

TEST_CASE("Memory - Cell fetch and store primitives (@ and !)", "[memory]") {
    pforth::Forth f;

    // ! ( n addr -- )
    f.push_ds(12345);
    f.push_ds(4000);
    f.word_store();
    REQUIRE(f.ds.empty());
    REQUIRE(f.fetch_cell(4000) == 12345);

    // @ ( addr -- n )
    f.push_ds(4000);
    f.word_fetch();
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 12345);

    // Underflow
    REQUIRE_THROWS_WITH(f.word_fetch(), "DATA STACK UNDERFLOW");
    f.push_ds(100);
    REQUIRE_THROWS_WITH(f.word_store(), "DATA STACK UNDERFLOW");
}

TEST_CASE("Memory - Byte fetch and store primitives (C@ and C!)", "[memory]") {
    pforth::Forth f;

    // C! ( c addr -- )
    f.push_ds(0x42);
    f.push_ds(5000);
    f.word_c_store();
    REQUIRE(f.ds.empty());
    REQUIRE(f.c_fetch(5000) == 0x42);

    // C@ ( addr -- c )
    f.push_ds(5000);
    f.word_c_fetch();
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.pop_ds() == 0x42);

    // Underflow
    REQUIRE_THROWS_WITH(f.word_c_fetch(), "DATA STACK UNDERFLOW");
    f.push_ds(100);
    REQUIRE_THROWS_WITH(f.word_c_store(), "DATA STACK UNDERFLOW");
}

TEST_CASE("Memory - Add to memory (+!)", "[memory]") {
    pforth::Forth f;

    // store 10 at 6000
    f.push_ds(10);
    f.push_ds(6000);
    f.word_store();

    // +! ( n addr -- ) add 25 to addr 6000
    f.push_ds(25);
    f.push_ds(6000);
    f.word_plus_store();

    REQUIRE(f.ds.empty());
    REQUIRE(f.fetch_cell(6000) == 35);

    // add -5
    f.push_ds(-5);
    f.push_ds(6000);
    f.word_plus_store();
    REQUIRE(f.fetch_cell(6000) == 30);
}

TEST_CASE("Memory - Question mark primitive (?)", "[memory]") {
    std::ostringstream oss;
    pforth::Forth f(nullptr, &oss);

    f.push_ds(9876);
    f.push_ds(7000);
    f.word_store();

    // ? ( addr -- )
    f.push_ds(7000);
    f.word_question();

    REQUIRE(f.ds.empty());
    REQUIRE(oss.str() == "9876 ");
}

