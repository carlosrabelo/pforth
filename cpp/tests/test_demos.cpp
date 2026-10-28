#include "catch.hpp"
#include "forth.hpp"
#include <sstream>

TEST_CASE("Demonstration Application - Hello World", "[demos]") {
    std::ostringstream out;
    pforth::Forth f(nullptr, &out);

    const std::string hello_src =
        ": HELLO\n"
        "  .\" Hello, world!\" CR\n"
        ";\n"
        "HELLO\n";

    f.load_system(hello_src);
    REQUIRE(out.str() == "Hello, world!\n");
}

TEST_CASE("Demonstration Application - Factorial calculation", "[demos]") {
    std::ostringstream out;
    pforth::Forth f(nullptr, &out);

    const std::string factorial_src =
        ": FACTORIAL ( n -- n! )\n"
        "  DUP 1 >\n"
        "  IF\n"
        "    DUP 1 - RECURSE *\n"
        "  THEN\n"
        ";\n"
        "5 FACTORIAL . CR\n"
        "7 FACTORIAL . CR\n";

    f.load_system(factorial_src);
    REQUIRE(out.str() == "120 \n5040 \n");
}

TEST_CASE("Demonstration Application - Fibonacci sequence", "[demos]") {
    std::ostringstream out;
    pforth::Forth f(nullptr, &out);

    const std::string fib_src =
        ": FIB ( n -- fib[n] )\n"
        "  0 1 ROT 0 DO\n"
        "    TUCK +\n"
        "  LOOP DROP\n"
        ";\n"
        "10 FIB . CR\n";

    f.load_system(fib_src);
    REQUIRE(out.str() == "55 \n");
}

TEST_CASE("Demonstration Application - Bubble sort in memory", "[demos]") {
    std::ostringstream out;
    pforth::Forth f(nullptr, &out);

    const std::string bubble_src =
        "CREATE DATA 9 C, 8 C, 7 C, 6 C, 5 C, 4 C, 3 C, 2 C, 1 C, 0 C,\n"
        ": PRINT\n"
        "  10 0 DO DATA I + C@ . LOOP CR\n"
        ";\n"
        ": BUBBLE ( addr len -- )\n"
        "  DUP 1 DO\n"
        "    DUP I - 0 DO\n"
        "      OVER I + DUP 1+\n"
        "      OVER C@ OVER C@\n"
        "      2DUP > IF\n"
        "        SWAP ROT C! SWAP C!\n"
        "      ELSE\n"
        "        2DROP 2DROP\n"
        "      THEN\n"
        "    LOOP\n"
        "  LOOP\n"
        "  2DROP\n"
        ";\n"
        "DATA 10 BUBBLE\n"
        "PRINT\n";

    f.load_system(bubble_src);
    REQUIRE(out.str() == "0 1 2 3 4 5 6 7 8 9 \n");
}

TEST_CASE("Demonstration Application - Sieve of Eratosthenes benchmark (1899 primes)", "[demos]") {
    std::ostringstream out;
    pforth::Forth f(nullptr, &out);

    const std::string sieve_src =
        "8190 CONSTANT SIZE\n"
        "VARIABLE FLAGS SIZE ALLOT\n"
        ": SETUP-FLAGS\n"
        "  SIZE 0 DO\n"
        "    1 FLAGS I + C!\n"
        "  LOOP ;\n"
        ": SIEVE\n"
        "  SETUP-FLAGS\n"
        "  0\n"
        "  SIZE 0 DO\n"
        "    FLAGS I + C@ IF\n"
        "      I I + 3 +\n"
        "      DUP I +\n"
        "      BEGIN\n"
        "        DUP SIZE <\n"
        "      WHILE\n"
        "        DUP FLAGS + 0 SWAP C!\n"
        "        OVER +\n"
        "      REPEAT\n"
        "      DROP DROP 1 +\n"
        "    THEN\n"
        "  LOOP\n"
        "  . CR\n"
        ";\n"
        "SIEVE\n";

    f.load_system(sieve_src);
    REQUIRE(out.str() == "1899 \n");
}

TEST_CASE("Demonstration Application - Tower of Hanoi recursion", "[demos]") {
    std::ostringstream out;
    pforth::Forth f(nullptr, &out);

    const std::string hanoi_src =
        "VARIABLE SRC\n"
        "VARIABLE DEST\n"
        "VARIABLE AUX\n"
        ": HANOI ( n src dest aux -- )\n"
        "  AUX ! DEST ! SRC !\n"
        "  DUP IF\n"
        "    SRC @ DEST @ AUX @\n"
        "    >R >R >R\n"
        "    DUP 1- SRC @ AUX @ DEST @ RECURSE\n"
        "    R> SRC ! R> DEST ! R> AUX !\n"
        "    .\" Move \" SRC @ . .\" -> \" DEST @ . CR\n"
        "    1- AUX @ DEST @ SRC @ RECURSE\n"
        "  ELSE\n"
        "    DROP\n"
        "  THEN\n"
        ";\n"
        "3 1 3 2 HANOI\n";

    f.load_system(hanoi_src);
    const std::string expected =
        "Move 1 -> 3 \n"
        "Move 1 -> 2 \n"
        "Move 3 -> 2 \n"
        "Move 1 -> 3 \n"
        "Move 2 -> 1 \n"
        "Move 2 -> 3 \n"
        "Move 1 -> 3 \n";
    REQUIRE(out.str() == expected);
}
