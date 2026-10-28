#include "forth.hpp"
#include <csignal>

namespace pforth {

static Forth* g_active_forth = nullptr;

static void sigint_handler(int) {
    if (g_active_forth) {
        g_active_forth->interrupted.store(1);
    }
}

Forth::Forth(std::istream* input, std::ostream* output)
    : in(input), out(output) {
    memory.fill(0);
}

Forth::~Forth() {
    remove_signal_handler();
}

void Forth::install_signal_handler() {
    g_active_forth = this;
    std::signal(SIGINT, sigint_handler);
}

void Forth::remove_signal_handler() {
    if (g_active_forth == this) {
        std::signal(SIGINT, SIG_DFL);
        g_active_forth = nullptr;
    }
}

void Forth::check_interrupt() {
    if (interrupted.load() != 0) {
        interrupted.store(0);
        abort();
        throw std::runtime_error("INTERRUPTED");
    }
}

void Forth::abort() {
    ds.clear();
    rs.clear();
    state = false;
}

void Forth::push_ds(Cell val) {
    ds.push_back(val);
}

Cell Forth::pop_ds() {
    if (ds.empty()) {
        throw std::runtime_error("DATA STACK UNDERFLOW");
    }
    Cell v = ds.back();
    ds.pop_back();
    return v;
}

Cell Forth::peek_ds() const {
    if (ds.empty()) {
        throw std::runtime_error("DATA STACK UNDERFLOW");
    }
    return ds.back();
}

void Forth::push_rs(Cell val) {
    rs.push_back(val);
}

Cell Forth::pop_rs() {
    if (rs.empty()) {
        throw std::runtime_error("RETURN STACK UNDERFLOW");
    }
    Cell v = rs.back();
    rs.pop_back();
    return v;
}

Cell Forth::peek_rs() const {
    if (rs.empty()) {
        throw std::runtime_error("RETURN STACK UNDERFLOW");
    }
    return rs.back();
}

void Forth::depth() {
    push_ds(static_cast<Cell>(ds.size()));
}

uint8_t Forth::c_fetch(uint32_t addr) const {
    if (addr >= memory.size()) {
        throw std::runtime_error("OUT OF MEMORY RANGE");
    }
    return memory[addr];
}

void Forth::c_store(uint32_t addr, uint8_t val) {
    if (addr >= memory.size()) {
        throw std::runtime_error("OUT OF MEMORY RANGE");
    }
    memory[addr] = val;
}

Cell Forth::fetch_cell(uint32_t addr) const {
    if (addr + 1 >= memory.size()) {
        throw std::runtime_error("OUT OF MEMORY RANGE");
    }
    uint16_t low = memory[addr];
    uint16_t high = memory[addr + 1];
    return static_cast<Cell>(low | (high << 8));
}

void Forth::store_cell(uint32_t addr, Cell val) {
    if (addr + 1 >= memory.size()) {
        throw std::runtime_error("OUT OF MEMORY RANGE");
    }
    memory[addr] = static_cast<uint8_t>(val & 0xFF);
    memory[addr + 1] = static_cast<uint8_t>((val >> 8) & 0xFF);
}

void Forth::word_fetch() {
    Cell addr = pop_ds();
    push_ds(fetch_cell(static_cast<uint16_t>(addr)));
}

void Forth::word_store() {
    Cell addr = pop_ds();
    Cell val = pop_ds();
    store_cell(static_cast<uint16_t>(addr), val);
}

void Forth::word_c_fetch() {
    Cell addr = pop_ds();
    push_ds(static_cast<Cell>(c_fetch(static_cast<uint16_t>(addr))));
}

void Forth::word_c_store() {
    Cell addr = pop_ds();
    Cell val = pop_ds();
    c_store(static_cast<uint16_t>(addr), static_cast<uint8_t>(val & 0xFF));
}

void Forth::word_plus_store() {
    Cell addr = pop_ds();
    Cell n = pop_ds();
    Cell cur = fetch_cell(static_cast<uint16_t>(addr));
    store_cell(static_cast<uint16_t>(addr), static_cast<Cell>(cur + n));
}

void Forth::word_question() {
    Cell addr = pop_ds();
    Cell val = fetch_cell(static_cast<uint16_t>(addr));
    if (out) {
        *out << val << " ";
    }
}

void Forth::word_fill() {
    Cell val = pop_ds();
    Cell len = pop_ds();
    Cell addr = pop_ds();
    uint8_t byte_val = static_cast<uint8_t>(val & 0xFF);
    uint32_t start = static_cast<uint16_t>(addr);
    for (int i = 0; i < len; ++i) {
        c_store(start + i, byte_val);
    }
}

void Forth::word_erase() {
    Cell len = pop_ds();
    Cell addr = pop_ds();
    uint32_t start = static_cast<uint16_t>(addr);
    for (int i = 0; i < len; ++i) {
        c_store(start + i, 0);
    }
}

void Forth::word_cmove() {
    Cell len = pop_ds();
    Cell src = pop_ds();
    Cell dst = pop_ds();
    uint32_t s = static_cast<uint16_t>(src);
    uint32_t d = static_cast<uint16_t>(dst);
    for (int i = 0; i < len; ++i) {
        c_store(d + i, c_fetch(s + i));
    }
}

void Forth::word_cmover() {
    Cell len = pop_ds();
    Cell src = pop_ds();
    Cell dst = pop_ds();
    uint32_t s = static_cast<uint16_t>(src);
    uint32_t d = static_cast<uint16_t>(dst);
    for (int i = len - 1; i >= 0; --i) {
        c_store(d + i, c_fetch(s + i));
    }
}

void Forth::word_dup() {
    push_ds(peek_ds());
}

void Forth::word_drop() {
    pop_ds();
}

void Forth::word_swap() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(b);
    push_ds(a);
}

void Forth::word_over() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(a);
    push_ds(b);
    push_ds(a);
}

void Forth::word_qdup() {
    Cell val = peek_ds();
    if (val != 0) {
        push_ds(val);
    }
}

void Forth::word_rot() {
    Cell c = pop_ds();
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(b);
    push_ds(c);
    push_ds(a);
}

void Forth::word_nrot() {
    Cell c = pop_ds();
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(c);
    push_ds(a);
    push_ds(b);
}

void Forth::word_nip() {
    Cell b = pop_ds();
    pop_ds(); // drops a
    push_ds(b);
}

void Forth::word_tuck() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(b);
    push_ds(a);
    push_ds(b);
}

void Forth::word_2dup() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(a);
    push_ds(b);
    push_ds(a);
    push_ds(b);
}

void Forth::word_2drop() {
    pop_ds();
    pop_ds();
}

void Forth::word_2swap() {
    Cell d = pop_ds();
    Cell c = pop_ds();
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(c);
    push_ds(d);
    push_ds(a);
    push_ds(b);
}

void Forth::word_2over() {
    Cell d = pop_ds();
    Cell c = pop_ds();
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(a);
    push_ds(b);
    push_ds(c);
    push_ds(d);
    push_ds(a);
    push_ds(b);
}

void Forth::word_to_r() {
    Cell val = pop_ds();
    push_rs(val);
}

void Forth::word_r_from() {
    Cell val = pop_rs();
    push_ds(val);
}

void Forth::word_r_fetch() {
    Cell val = peek_rs();
    push_ds(val);
}

void Forth::word_plus() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(static_cast<Cell>(a + b));
}

void Forth::word_minus() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(static_cast<Cell>(a - b));
}

void Forth::word_multiply() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(static_cast<Cell>(a * b));
}

void Forth::word_slash_mod() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    if (b == 0) {
        throw std::runtime_error("DIVISION BY ZERO");
    }
    push_ds(static_cast<Cell>(a % b));
    push_ds(static_cast<Cell>(a / b));
}

void Forth::word_slash() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    if (b == 0) {
        throw std::runtime_error("DIVISION BY ZERO");
    }
    push_ds(static_cast<Cell>(a / b));
}

void Forth::word_mod() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    if (b == 0) {
        throw std::runtime_error("DIVISION BY ZERO");
    }
    push_ds(static_cast<Cell>(a % b));
}

void Forth::word_negate() {
    Cell n = pop_ds();
    push_ds(static_cast<Cell>(-n));
}

void Forth::word_one_plus() {
    Cell n = pop_ds();
    push_ds(static_cast<Cell>(n + 1));
}

void Forth::word_one_minus() {
    Cell n = pop_ds();
    push_ds(static_cast<Cell>(n - 1));
}

void Forth::word_two_plus() {
    Cell n = pop_ds();
    push_ds(static_cast<Cell>(n + 2));
}

void Forth::word_two_minus() {
    Cell n = pop_ds();
    push_ds(static_cast<Cell>(n - 2));
}

void Forth::word_two_star() {
    Cell n = pop_ds();
    push_ds(static_cast<Cell>(n << 1));
}

void Forth::word_two_slash() {
    Cell n = pop_ds();
    push_ds(static_cast<Cell>(n >> 1));
}

void Forth::word_abs() {
    Cell n = pop_ds();
    push_ds(n < 0 ? static_cast<Cell>(-n) : n);
}

void Forth::word_min() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(a < b ? a : b);
}

void Forth::word_max() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(a > b ? a : b);
}

void Forth::key() {
    char c = 0;
    if (in && in->get(c)) {
        push_ds(static_cast<Cell>(static_cast<unsigned char>(c)));
    } else {
        push_ds(0);
    }
}

void Forth::emit() {
    Cell c = pop_ds();
    if (out) {
        out->put(static_cast<char>(c & 0xFF));
    }
}

void Forth::cr() {
    if (out) {
        out->put('\n');
    }
}

void Forth::space() {
    if (out) {
        out->put(' ');
    }
}

void Forth::spaces() {
    Cell n = pop_ds();
    if (out && n > 0) {
        for (Cell i = 0; i < n; ++i) {
            out->put(' ');
        }
    }
}

void Forth::type() {
    Cell len = pop_ds();
    Cell addr = pop_ds();
    if (out && len > 0) {
        uint16_t uaddr = static_cast<uint16_t>(addr);
        for (Cell i = 0; i < len; ++i) {
            uint32_t curr = static_cast<uint32_t>(uaddr) + i;
            if (curr < memory.size()) {
                out->put(static_cast<char>(memory[curr]));
            }
        }
    }
}

void Forth::bye() {
    running = false;
}

std::string Forth::read_line() {
    std::string line;
    if (!in || !std::getline(*in, line)) {
        running = false;
        return "";
    }
    if (!line.empty() && line.back() == '\r') {
        line.pop_back();
    }
    return line;
}

void Forth::expect() {
    Cell maxlen = pop_ds();
    Cell addr = pop_ds();
    std::string line = read_line();

    size_t count = line.size();
    if (maxlen > 0 && count > static_cast<size_t>(maxlen)) {
        count = static_cast<size_t>(maxlen);
    }

    uint16_t uaddr = static_cast<uint16_t>(addr);
    for (size_t i = 0; i < count; ++i) {
        c_store(static_cast<uint32_t>(uaddr) + i, static_cast<uint8_t>(line[i]));
    }
    if (static_cast<size_t>(uaddr) + count < memory.size()) {
        c_store(static_cast<uint32_t>(uaddr) + count, 0);
    }

    // SPAN variable is stored at address 1000 (little-endian 16-bit cell)
    store_cell(1000, static_cast<Cell>(count));
}

void Forth::span() {
    push_ds(1000);
}

void Forth::interpret_line(const std::string& line) {
    (void)line;
}

void Forth::repl_step() {
    try {
        if (!state && out) {
            *out << " ok\n";
        }
        std::string line = read_line();
        if (!running) {
            return;
        }
        check_interrupt();
        interpret_line(line);
    } catch (const std::exception& e) {
        if (out) {
            *out << "\n" << e.what() << "\n";
        }
        abort();
    }
}

void Forth::quit() {
    install_signal_handler();
    running = true;
    abort();
    while (running) {
        repl_step();
    }
}

} // namespace pforth
