#include "catch.hpp"
#include "forth.hpp"

TEST_CASE("Stacks - Data Stack basic push, pop, peek", "[stacks]") {
    pforth::Forth f;
    REQUIRE(f.ds.empty());

    f.push_ds(10);
    f.push_ds(20);
    REQUIRE(f.ds.size() == 2);
    REQUIRE(f.peek_ds() == 20);

    REQUIRE(f.pop_ds() == 20);
    REQUIRE(f.pop_ds() == 10);
    REQUIRE(f.ds.empty());
}

TEST_CASE("Stacks - Data Stack underflow raises error", "[stacks]") {
    pforth::Forth f;
    REQUIRE_THROWS_WITH(f.pop_ds(), "DATA STACK UNDERFLOW");
    REQUIRE_THROWS_WITH(f.peek_ds(), "DATA STACK UNDERFLOW");
}

TEST_CASE("Stacks - Return Stack push, pop, peek, underflow", "[stacks]") {
    pforth::Forth f;
    REQUIRE(f.rs.empty());

    f.push_rs(500);
    f.push_rs(600);
    REQUIRE(f.peek_rs() == 600);

    REQUIRE(f.pop_rs() == 600);
    REQUIRE(f.pop_rs() == 500);
    REQUIRE(f.rs.empty());

    REQUIRE_THROWS_WITH(f.pop_rs(), "RETURN STACK UNDERFLOW");
    REQUIRE_THROWS_WITH(f.peek_rs(), "RETURN STACK UNDERFLOW");
}

TEST_CASE("Stacks - DEPTH pushes current count of DS", "[stacks]") {
    pforth::Forth f;
    f.depth();
    REQUIRE(f.pop_ds() == 0);

    f.push_ds(1);
    f.push_ds(2);
    f.push_ds(3);
    f.depth();
    REQUIRE(f.pop_ds() == 3);
    REQUIRE(f.ds.size() == 3);
}
