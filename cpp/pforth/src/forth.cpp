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
