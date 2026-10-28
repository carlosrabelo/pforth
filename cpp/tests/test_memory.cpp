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

TEST_CASE("Memory - FILL and ERASE operations", "[memory]") {
    pforth::Forth f;

    // FILL ( addr len c -- )
    f.push_ds(8000); // addr
    f.push_ds(10);   // len
    f.push_ds(0xEE); // c
    f.word_fill();

    REQUIRE(f.ds.empty());
    for (int i = 0; i < 10; ++i) {
        REQUIRE(f.c_fetch(8000 + i) == 0xEE);
    }

    // ERASE ( addr len -- )
    f.push_ds(8000); // addr
    f.push_ds(5);    // len
    f.word_erase();

    REQUIRE(f.ds.empty());
    for (int i = 0; i < 5; ++i) {
        REQUIRE(f.c_fetch(8000 + i) == 0);
    }
    for (int i = 5; i < 10; ++i) {
        REQUIRE(f.c_fetch(8000 + i) == 0xEE);
    }
}

TEST_CASE("Memory - CMOVE and CMOVE> operations", "[memory]") {
    pforth::Forth f;

    // Setup source bytes 10..14 at 9000
    for (int i = 0; i < 5; ++i) {
        f.c_store(9000 + i, static_cast<uint8_t>(i + 10));
    }

    // CMOVE non-overlapping: dst=9010, src=9000, len=5
    // Forth stack convention: ( dst src len -- )
    f.push_ds(9010); // dst
    f.push_ds(9000); // src
    f.push_ds(5);    // len
    f.word_cmove();

    REQUIRE(f.ds.empty());
    for (int i = 0; i < 5; ++i) {
        REQUIRE(f.c_fetch(9010 + i) == static_cast<uint8_t>(i + 10));
    }

    // Overlapping forward CMOVE with dst > src (propagates first byte)
    // Setup bytes at 9020..9024: [20, 21, 22, 23, 24]
    for (int i = 0; i < 5; ++i) {
        f.c_store(9020 + i, static_cast<uint8_t>(i + 20));
    }
    // dst=9021, src=9020, len=4
    f.push_ds(9021); // dst
    f.push_ds(9020); // src
    f.push_ds(4);    // len
    f.word_cmove();

    // Since forward copies 9020->9021 first, 9021 becomes 20, then 9021->9022 copies 20, etc.
    REQUIRE(f.c_fetch(9021) == 20);
    REQUIRE(f.c_fetch(9022) == 20);
    REQUIRE(f.c_fetch(9023) == 20);
    REQUIRE(f.c_fetch(9024) == 20);

    // Overlapping backward CMOVE> with dst > src (correctly preserves values)
    // Setup bytes at 9030..9034: [30, 31, 32, 33, 34]
    for (int i = 0; i < 5; ++i) {
        f.c_store(9030 + i, static_cast<uint8_t>(i + 30));
    }
    // dst=9031, src=9030, len=4
    f.push_ds(9031); // dst
    f.push_ds(9030); // src
    f.push_ds(4);    // len
    f.word_cmover();

    // Backward copies 9033->9034 (33), 9032->9033 (32), 9031->9032 (31), 9030->9031 (30)
    REQUIRE(f.c_fetch(9031) == 30);
    REQUIRE(f.c_fetch(9032) == 31);
    REQUIRE(f.c_fetch(9033) == 32);
    REQUIRE(f.c_fetch(9034) == 33);
}


