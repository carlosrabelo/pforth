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

} // namespace pforth
