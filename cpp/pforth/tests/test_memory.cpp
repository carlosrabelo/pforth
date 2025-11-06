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
