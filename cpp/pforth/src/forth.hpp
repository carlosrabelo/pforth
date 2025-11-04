#pragma once

#include <array>
#include <cstdint>
#include <iostream>
#include <stdexcept>
#include <vector>

namespace pforth {

using Cell = std::int16_t;

class Forth {
public:
    std::vector<Cell> ds;
    std::vector<Cell> rs;
    std::array<std::uint8_t, 65536> memory{};
    bool running{true};

    std::istream* in{&std::cin};
    std::ostream* out{&std::cout};

    Forth(std::istream* input = &std::cin, std::ostream* output = &std::cout);

    void push_ds(Cell val);
    Cell pop_ds();
    Cell peek_ds() const;

    void key();
    void emit();
};

} // namespace pforth
