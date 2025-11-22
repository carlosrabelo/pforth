#include "catch.hpp"
#include "forth.hpp"
#include <sstream>

TEST_CASE("Advanced I/O - Numeric output and stack inspector (., U., .S, PAGE)", "[advanced_io]") {
    std::ostringstream out;
    std::istringstream in;
    pforth::Forth f(&in, &out);

    // . in base 10
    f.interpret_line("42 .");
    REQUIRE(out.str() == "42 ");
    out.str("");

    f.interpret_line("-123 .");
    REQUIRE(out.str() == "-123 ");
    out.str("");

    f.interpret_line("0 .");
    REQUIRE(out.str() == "0 ");
    out.str("");

    // . in base 16 (hex)
    f.interpret_line("255 16 BASE ! . A BASE !");
    REQUIRE(out.str() == "FF ");
    REQUIRE(f.get_base() == 10);
    out.str("");

    // U. unsigned output
    f.interpret_line("-1 U.");
    REQUIRE(out.str() == "65535 ");
    out.str("");

    f.interpret_line("30000 U.");
    REQUIRE(out.str() == "30000 ");
    out.str("");

    // .S non-destructive stack inspector
    f.interpret_line(".S");
    REQUIRE(out.str() == "<0> ");
    out.str("");

    f.interpret_line("10 20 30 .S");
    REQUIRE(out.str() == "<3> 10 20 30 ");
    REQUIRE(f.ds.size() == 3);
    REQUIRE(f.ds[0] == 10);
    REQUIRE(f.ds[1] == 20);
    REQUIRE(f.ds[2] == 30);
    out.str("");
    f.ds.clear();

    // PAGE screen clear
    f.interpret_line("PAGE");
    REQUIRE(out.str() == "\033[2J\033[H");
}
