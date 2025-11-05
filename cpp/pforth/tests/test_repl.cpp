#include "catch.hpp"
#include "forth.hpp"
#include <sstream>

TEST_CASE("REPL - Interactive prompt displays ' ok\\n'", "[repl]") {
    std::stringstream in("line 1\nline 2\n");
    std::stringstream out;

    pforth::Forth f(&in, &out);
    f.quit();

    std::string output = out.str();
    REQUIRE(output.find(" ok\n") != std::string::npos);
    REQUIRE(f.running == false); // Clean termination on EOF
}

TEST_CASE("REPL - Interrupt handling resets stacks and recovers cleanly", "[repl]") {
    std::stringstream in("dummy\n");
    std::stringstream out;

    pforth::Forth f(&in, &out);
    f.push_ds(42);
    f.push_rs(100);

    // Simulate SIGINT interrupt
    f.interrupted.store(1);

    // check_interrupt throws exception but cleans stacks
    REQUIRE_THROWS_WITH(f.check_interrupt(), "INTERRUPTED");
    REQUIRE(f.ds.empty());
    REQUIRE(f.rs.empty());
    REQUIRE(f.interrupted.load() == 0);

    // Test that repl_step handles interrupt smoothly
    std::stringstream in2("line\n");
    std::stringstream out2;
    pforth::Forth f2(&in2, &out2);
    f2.push_ds(99);
    f2.interrupted.store(1);

    // Does not throw out of repl_step; recovers and stays alive
    f2.repl_step();
    REQUIRE(f2.ds.empty());
    REQUIRE(f2.running == true);
    REQUIRE(out2.str().find("INTERRUPTED") != std::string::npos);
}
