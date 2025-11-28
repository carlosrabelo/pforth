#include "catch.hpp"
#include "forth.hpp"
#include <cstdlib>
#include <filesystem>
#include <sstream>
#include <string>

namespace {

// Resolve demos/<name>.fs: $PFORTH_DEMOS, else walk up from cwd.
std::string demo_path(const std::string& name) {
    const std::string file = name + ".fs";
    std::error_code ec;
    if (const char* env = std::getenv("PFORTH_DEMOS"); env != nullptr && env[0] != '\0') {
        std::filesystem::path candidate = std::filesystem::path(env) / file;
        if (std::filesystem::is_regular_file(candidate, ec)) {
            return candidate.string();
        }
    }
    std::filesystem::path dir = std::filesystem::current_path(ec);
    while (!dir.empty()) {
        std::filesystem::path candidate = dir / "demos" / file;
        if (std::filesystem::is_regular_file(candidate, ec)) {
            return candidate.string();
        }
        if (!dir.has_parent_path() || dir.parent_path() == dir) {
            break;
        }
        dir = dir.parent_path();
    }
    FAIL("demo file not found: demos/" << file);
    return "";
}

std::string run_demo(const std::string& name) {
    std::ostringstream out;
    pforth::Forth f(nullptr, &out);
    f.load_file(demo_path(name));
    return out.str();
}

} // namespace

TEST_CASE("Demonstration Application - Hello World", "[demos]") {
    REQUIRE(run_demo("hello") == "Hello, world!\n");
}

TEST_CASE("Demonstration Application - Factorial calculation", "[demos]") {
    REQUIRE(run_demo("factorial") == "120 \n5040 \n");
}

TEST_CASE("Demonstration Application - Fibonacci sequence", "[demos]") {
    REQUIRE(run_demo("fibonacci") == "55 \n");
}

TEST_CASE("Demonstration Application - Bubble sort in memory", "[demos]") {
    REQUIRE(run_demo("bubble_sort") == "0 1 2 3 4 5 6 7 8 9 \n");
}

TEST_CASE("Demonstration Application - Sieve of Eratosthenes benchmark (1899 primes)", "[demos]") {
    REQUIRE(run_demo("sieve") == "1899 \n");
}

TEST_CASE("Demonstration Application - Tower of Hanoi recursion", "[demos]") {
    const std::string expected =
        "Move 1 -> 3 \n"
        "Move 1 -> 2 \n"
        "Move 3 -> 2 \n"
        "Move 1 -> 3 \n"
        "Move 2 -> 1 \n"
        "Move 2 -> 3 \n"
        "Move 1 -> 3 \n";
    REQUIRE(run_demo("hanoi") == expected);
}
