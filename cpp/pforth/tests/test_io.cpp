#define CATCH_CONFIG_MAIN
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
