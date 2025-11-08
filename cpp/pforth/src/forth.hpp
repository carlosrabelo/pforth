#pragma once

#include <array>
#include <atomic>
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
    bool state{false}; // false: interpreting, true: compiling
    std::atomic<int> interrupted{0};

    // Text Input Buffer & input cursor
    std::string tib;
    size_t in_ptr{0};

    std::istream* in{&std::cin};
    std::ostream* out{&std::cout};

    Forth(std::istream* input = &std::cin, std::ostream* output = &std::cout);
    ~Forth();

    // Spec 002.1 Data and Return Stack Primitives
    void push_ds(Cell val);
    Cell pop_ds();
    Cell peek_ds() const;

    void push_rs(Cell val);
    Cell pop_rs();
    Cell peek_rs() const;
    void depth();

    // Spec 002.2 64KB Linear RAM Memory Model
    uint8_t c_fetch(uint32_t addr) const;
    void c_store(uint32_t addr, uint8_t val);
    Cell fetch_cell(uint32_t addr) const;
    void store_cell(uint32_t addr, Cell val);

    // Spec 002.3 Cell and Byte Memory Access Primitives
    void word_fetch();      // @
    void word_store();      // !
    void word_c_fetch();    // C@
    void word_c_store();    // C!
    void word_plus_store(); // +!
    void word_question();   // ?

    // Spec 002.4 Memory Block Operations
    void word_fill();   // FILL
    void word_erase();  // ERASE
    void word_cmove();  // CMOVE
    void word_cmover(); // CMOVE>

    // Spec 003.1 Basic Stack Manipulation Words
    void word_dup();  // DUP
    void word_drop(); // DROP
    void word_swap(); // SWAP
    void word_over(); // OVER

    // Spec 003.2 Extended Single Stack Manipulation Words
    void word_qdup();  // ?DUP
    void word_rot();   // ROT
    void word_nrot();  // -ROT
    void word_nip();   // NIP
    void word_tuck();  // TUCK

    // Spec 003.3 Double Cell Stack Manipulation Words
    void word_2dup();  // 2DUP
    void word_2drop(); // 2DROP
    void word_2swap(); // 2SWAP
    void word_2over(); // 2OVER

    // Spec 003.4 Return Stack Transfer Words
    void word_to_r();    // >R
    void word_r_from();  // R>
    void word_r_fetch(); // R@

    // Spec 001.2 Console I/O Primitives
    void key();
    void emit();
    void cr();
    void space();
    void spaces();
    void type();
    void bye();

    // Spec 001.3 Line Reader and Text Input Buffer
    std::string read_line();
    void expect();
    void span();

    // Spec 001.4 Interactive REPL with Interrupt Handling
    void install_signal_handler();
    void remove_signal_handler();
    void check_interrupt();
    void abort();
    void repl_step();
    void quit();
    virtual void interpret_line(const std::string& line);
};

} // namespace pforth
