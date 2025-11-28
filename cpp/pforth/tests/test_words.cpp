#include "catch.hpp"
#include "forth.hpp"
#include <sstream>

TEST_CASE("System tools - WORDS and DUMP", "[words]") {
    std::stringstream in;
    std::stringstream out;
    pforth::Forth f(&in, &out);

    f.interpret_line(": AAA 1 ;");
    f.interpret_line(": BBB 2 ; IMMEDIATE");
    f.interpret_line("WORDS");

    std::string listing = out.str();
    REQUIRE(listing.rfind("I:BBB AAA ", 0) == 0);
    REQUIRE(listing.back() == '\n');
    out.str("");
    out.clear();

    for (int i = 0; i < 16; ++i) {
        f.memory[0x3000 + i] = static_cast<uint8_t>(0x41 + i);
    }
    f.interpret_line("12288 16 DUMP");
    REQUIRE(out.str() == "3000  41 42 43 44 45 46 47 48 49 4A 4B 4C 4D 4E 4F 50  ABCDEFGHIJKLMNOP\n");
    out.str("");
    out.clear();

    f.interpret_line("0 0 DUMP");
    f.interpret_line("100 -1 DUMP");
    REQUIRE(out.str().empty());

    REQUIRE_THROWS_WITH(f.interpret_line("65530 10 DUMP"), "OUT OF MEMORY RANGE");
    REQUIRE(out.str() == "FFFA  00 00 00 00 00 00  ......\n");
}
