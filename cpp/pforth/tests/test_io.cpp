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

TEST_CASE("Line reader - EXPECT reads line, stores in memory, updates SPAN", "[io]") {
    std::stringstream in("TEST LINE\n");
    std::stringstream out;

    pforth::Forth f(&in, &out);

    // Buffer at 2000, maxlen 20
    f.push_ds(2000);
    f.push_ds(20);
    f.expect();

    // Verify memory contains "TEST LINE"
    std::string result;
    for (int i = 0; i < 9; ++i) {
        result.push_back(static_cast<char>(f.memory[2000 + i]));
    }
    REQUIRE(result == "TEST LINE");

    // Check SPAN variable address and value at 1000
    f.span();
    REQUIRE(f.pop_ds() == 1000);
    uint16_t span_val = f.memory[1000] | (f.memory[1001] << 8);
    REQUIRE(span_val == 9);
}

TEST_CASE("Line reader - EXPECT truncates to maxlen", "[io]") {
    std::stringstream in("A VERY LONG STRING THAT EXCEEDS CAPACITY\n");
    std::stringstream out;

    pforth::Forth f(&in, &out);

    // Buffer at 3000, maxlen 5
    f.push_ds(3000);
    f.push_ds(5);
    f.expect();

    std::string result;
    for (int i = 0; i < 5; ++i) {
        result.push_back(static_cast<char>(f.memory[3000 + i]));
    }
    REQUIRE(result == "A VER");

    uint16_t span_val = f.memory[1000] | (f.memory[1001] << 8);
    REQUIRE(span_val == 5);
}
