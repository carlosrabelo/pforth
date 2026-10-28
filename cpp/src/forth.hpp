#pragma once

#include <array>
#include <cstdint>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

namespace pforth {

using Cell = std::int16_t;

class Forth {
public:
    std::vector<Cell> ds;
    std::vector<Cell> rs;
    std::array<std::uint8_t, 65536> memory{};
    int dp{1024};
    bool running{true};

    std::istream* in{&std::cin};
    std::ostream* out{&std::cout};

    Forth(std::istream* input = &std::cin, std::ostream* output = &std::cout);

    void push_ds(Cell val);
    Cell pop_ds();
    Cell peek_ds() const;

    void push_rs(Cell val);
    Cell pop_rs();
    Cell peek_rs() const;

    // Spec 001.2 Console I/O Primitives
    void key();
    void emit();
    void cr();
    void space();
    void spaces();
    void type();
    void bye();
};

} // namespace pforth
