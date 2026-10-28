#include "catch.hpp"
#include "forth.hpp"
#include <sstream>

TEST_CASE("Console I/O - EMIT and KEY", "[io]") {
    std::stringstream in("X");
    std::stringstream out;

    pforth::Forth f(&in, &out);

    f.key();
    REQUIRE(f.ds.size() == 1);
    REQUIRE(f.peek_ds() == static_cast<pforth::Cell>('X'));

    f.emit();
    REQUIRE(f.ds.empty());
    REQUIRE(out.str() == "X");
}

TEST_CASE("Console I/O - CR, SPACE, SPACES", "[io]") {
    std::stringstream in;
    std::stringstream out;

    pforth::Forth f(&in, &out);

    f.cr();
    f.space();
    f.push_ds(3);
    f.spaces();

    REQUIRE(out.str() == "\n    ");
}

TEST_CASE("Console I/O - TYPE prints memory buffer", "[io]") {
    std::stringstream in;
    std::stringstream out;

    pforth::Forth f(&in, &out);
    const char text[] = "HELLO";
    for (size_t i = 0; i < 5; ++i) {
        f.memory[1024 + i] = static_cast<uint8_t>(text[i]);
    }

    f.push_ds(1024);
    f.push_ds(5);
    f.type();

    REQUIRE(out.str() == "HELLO");
    REQUIRE(f.ds.empty());
}

TEST_CASE("Console I/O - BYE sets running flag to false", "[io]") {
    pforth::Forth f;
    REQUIRE(f.running == true);
    f.bye();
    REQUIRE(f.running == false);
}
