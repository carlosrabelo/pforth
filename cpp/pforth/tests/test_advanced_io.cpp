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

TEST_CASE("Advanced I/O - String literals and output (.\", S\", comments)", "[advanced_io]") {
    std::ostringstream out;
    std::istringstream in;
    pforth::Forth f(&in, &out);

    // ." in interpret mode
    f.interpret_line(".\" Hello, World!\"");
    REQUIRE(out.str() == "Hello, World!");
    out.str("");

    // ." inside colon definition
    f.interpret_line(": GREET .\" Welcome to pForth!\" ;");
    f.interpret_line("GREET");
    REQUIRE(out.str() == "Welcome to pForth!");
    out.str("");

    // S" in interpret mode
    f.interpret_line("S\" Forth String\"");
    REQUIRE(f.ds.size() == 2);
    pforth::Cell s_len = f.pop_ds();
    pforth::Cell s_addr = f.pop_ds();
    REQUIRE(s_len == 12);
    std::string s_read;
    for (int i = 0; i < s_len; ++i) {
        s_read += static_cast<char>(f.c_fetch(s_addr + i));
    }
    REQUIRE(s_read == "Forth String");

    // S" inside colon definition
    f.interpret_line(": PUSH_STR S\" Comp String\" ;");
    f.interpret_line("PUSH_STR");
    REQUIRE(f.ds.size() == 2);
    s_len = f.pop_ds();
    s_addr = f.pop_ds();
    REQUIRE(s_len == 11);
    s_read.clear();
    for (int i = 0; i < s_len; ++i) {
        s_read += static_cast<char>(f.c_fetch(s_addr + i));
    }
    REQUIRE(s_read == "Comp String");

    // ( paren comment
    f.interpret_line("10 ( comment in middle ) 20 +");
    REQUIRE(f.pop_ds() == 30);

    // \ backslash comment to end of line
    f.interpret_line("50 50 + \\ trailing comment should ignore 999");
    REQUIRE(f.pop_ds() == 100);
    REQUIRE(f.ds.empty());
}
