#include "forth.hpp"

namespace pforth {

Forth::Forth(std::istream* input, std::ostream* output)
    : in(input), out(output) {
    memory.fill(0);
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
        if (static_cast<size_t>(uaddr) + i < memory.size()) {
            memory[uaddr + i] = static_cast<uint8_t>(line[i]);
        }
    }
    if (static_cast<size_t>(uaddr) + count < memory.size()) {
        memory[uaddr + count] = 0;
    }

    // SPAN variable is stored at address 1000 (little-endian 16-bit cell)
    memory[1000] = static_cast<uint8_t>(count & 0xFF);
    memory[1001] = static_cast<uint8_t>((count >> 8) & 0xFF);
}

void Forth::span() {
    push_ds(1000);
}

} // namespace pforth
