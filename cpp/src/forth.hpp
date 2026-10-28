#pragma once

#include <array>
#include <atomic>
#include <cstdint>
#include <functional>
#include <iostream>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <unordered_set>
#include <vector>

namespace pforth {

using Cell = std::int16_t;

enum class WordType {
    Primitive,
    Colon,
    Constant,
    Variable,
    Create
};

class Forth;
using XTCode = std::function<void(Forth*)>;

struct Word {
    std::string name;
    bool immediate{false};
    WordType type{WordType::Primitive};
    XTCode code;
    std::vector<Cell> body;
    int pfa{0};
};

class Forth {
public:
    std::vector<Cell> ds;
    std::vector<Cell> rs;
    std::array<std::uint8_t, 65536> memory{};
    int dp{1024};
    bool running{true};
    bool state{false}; // false: interpreting, true: compiling
    std::atomic<int> interrupted{0};

    // Execution frame & compilation list
    std::vector<Cell> current_body;
    int ip{0};
    std::vector<Cell> compile_list;
    int compiling_xt{-1};

    // Dictionary
    std::vector<Word> words;
    std::unordered_map<std::string, int> words_by_name;
    int latest{-1};
    std::unordered_set<std::string> loaded_modules;

    // Text Input Buffer & input cursor
    std::string tib;
    size_t in_ptr{0};

    std::istream* in{&std::cin};
    std::ostream* out{&std::cout};

    Forth(std::istream* input = &std::cin, std::ostream* output = &std::cout);
    ~Forth();

    // Dictionary management
    Word* define_word(const std::string& name, WordType type, XTCode code, const std::vector<Cell>& body = {}, bool immediate = false);
    Word* define_primitive(const std::string& name, XTCode code);
    Word* define_immediate(const std::string& name, XTCode code);
    Word* lookup(const std::string& name);
    int find_xt(const std::string& name) const;
    Word* latest_word();
    void init_primitives();
    bool parse_word(std::string& word_out);

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

    // Spec 004.1 Basic Arithmetic and Division Primitives
    void word_plus();      // +
    void word_minus();     // -
    void word_multiply();  // *
    void word_slash_mod(); // /MOD
    void word_slash();     // /
    void word_mod();       // MOD
    void word_negate();    // NEGATE

    // Spec 004.2 Arithmetic Scaling and Helper Words
    void word_one_plus();  // 1+
    void word_one_minus(); // 1-
    void word_two_plus();  // 2+
    void word_two_minus(); // 2-
    void word_two_star();  // 2*
    void word_two_slash(); // 2/
    void word_abs();       // ABS
    void word_min();       // MIN
    void word_max();       // MAX

    // Spec 004.3 Bitwise Logic Primitives
    void word_and();    // AND
    void word_or();     // OR
    void word_xor();    // XOR
    void word_invert(); // INVERT

    // Spec 004.4 Comparison and Equality Primitives
    void word_equals();          // =
    void word_not_equals();      // <>
    void word_less();            // <
    void word_greater();         // >
    void word_less_equal();      // <=
    void word_greater_equal();   // >=
    void word_zero_equals();     // 0=
    void word_zero_not_equals(); // 0<>
    void word_zero_less();       // 0<
    void word_zero_greater();    // 0>
    void word_u_less();          // U<
    void word_u_greater();       // U>
    void word_u_less_equal();    // U<=
    void word_u_greater_equal(); // U>=

    // Spec 005.1 Dictionary Entry Structure and Lookup
    void word_find();    // FIND
    void word_tick();    // '
    void word_to_body(); // >BODY

    // Spec 005.2 Dictionary Memory Allocation Pointers
    Cell here() const;
    void allot(int n);
    void word_dp();      // DP
    void word_here();    // HERE
    void word_pad();     // PAD
    void word_allot();   // ALLOT
    void word_comma();   // ,
    void word_c_comma(); // C,

    // Spec 005.3 Execution Token Dispatcher
    void execute_word(int xt);
    void word_execute(); // EXECUTE

    // Spec 005.4 Defining Words
    void word_create();   // CREATE
    void word_constant(); // CONSTANT
    void word_variable(); // VARIABLE

    // Spec 006.1 Delimited Word Scanner and Numeric Parser
    int get_base() const;
    void set_base(int b);
    bool parse_number(const std::string& token, Cell& out_val) const;
    void word_word();   // WORD
    void word_number(); // NUMBER
    void word_base();   // BASE

    // Spec 006.2 Outer Text Interpreter Loop
    void interpret_loop();
    void word_interpret(); // INTERPRET
    void word_lit();       // LIT

    // Spec 006.3 Compiler State & Directives
    void word_state();           // STATE
    void word_left_bracket();    // [
    void word_right_bracket();   // ]
    void word_literal();         // LITERAL
    void word_compile();         // COMPILE
    void word_bracket_compile(); // [COMPILE]

    // Spec 006.4 Colon Definition Compiler and Immediate Flag
    void word_colon();     // :
    void word_semicolon(); // ;
    void word_exit();      // EXIT
    void word_immediate(); // IMMEDIATE
    void word_recurse();   // RECURSE

    // Spec 007.1 Runtime Branch Execution
    void word_branch();      // BRANCH
    void word_zero_branch(); // 0BRANCH

    // Spec 007.2 Conditional Branching Compiler
    void word_if();   // IF
    void word_then(); // THEN
    void word_else(); // ELSE

    // Spec 007.3 Indefinite Loop Constructs
    void word_begin();  // BEGIN
    void word_until();  // UNTIL
    void word_again();  // AGAIN
    void word_while();  // WHILE
    void word_repeat(); // REPEAT

    // Spec 007.4 Counted Loop Runtime and Compiler
    void word_do_run();        // (DO)
    void word_loop_run();      // (LOOP)
    void word_plus_loop_run(); // (+LOOP)
    void word_leave_run();     // (LEAVE)
    void word_do();            // DO
    void word_loop();          // LOOP
    void word_plus_loop();     // +LOOP
    void word_i();             // I
    void word_j();             // J
    void word_leave();         // LEAVE
    void word_unloop();        // UNLOOP

    // Spec 008.1 Numeric Output and Stack Inspector
    void word_dot();   // .
    void word_u_dot(); // U.
    void word_dot_s(); // .S
    void word_page();  // PAGE

    // Spec 008.2 String Literals and Output
    std::string parse_string();
    void word_dot_string(); // ."
    void word_s_string();   // S"
    void word_paren();      // (
    void word_backslash();  // '\'

    // Spec 008.3 String Evaluation and File Loading
    void load_file(const std::string& path);
    void load_system(const std::string& data);
    void require_module(const std::string& module_name);
    void word_evaluate(); // EVALUATE
    void word_included(); // INCLUDED
    void word_include();  // INCLUDE

    // Spec 008.4 Module Dependency Resolver
    void word_require(); // #REQUIRE

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
