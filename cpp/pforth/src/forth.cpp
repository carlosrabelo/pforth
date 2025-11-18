#include "forth.hpp"
#include <cctype>
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
    store_cell(1002, 1024);
    dp = 1024;
    store_cell(1004, 10);
    init_primitives();
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
    compile_list.clear();
    state = false;
    in_ptr = tib.size();
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

void Forth::word_and() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(static_cast<Cell>(a & b));
}

void Forth::word_or() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(static_cast<Cell>(a | b));
}

void Forth::word_xor() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(static_cast<Cell>(a ^ b));
}

void Forth::word_invert() {
    Cell n = pop_ds();
    push_ds(static_cast<Cell>(~n));
}

void Forth::word_equals() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(a == b ? -1 : 0);
}

void Forth::word_not_equals() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(a != b ? -1 : 0);
}

void Forth::word_less() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(a < b ? -1 : 0);
}

void Forth::word_greater() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(a > b ? -1 : 0);
}

void Forth::word_less_equal() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(a <= b ? -1 : 0);
}

void Forth::word_greater_equal() {
    Cell b = pop_ds();
    Cell a = pop_ds();
    push_ds(a >= b ? -1 : 0);
}

void Forth::word_zero_equals() {
    Cell n = pop_ds();
    push_ds(n == 0 ? -1 : 0);
}

void Forth::word_zero_not_equals() {
    Cell n = pop_ds();
    push_ds(n != 0 ? -1 : 0);
}

void Forth::word_zero_less() {
    Cell n = pop_ds();
    push_ds(n < 0 ? -1 : 0);
}

void Forth::word_zero_greater() {
    Cell n = pop_ds();
    push_ds(n > 0 ? -1 : 0);
}

void Forth::word_u_less() {
    uint16_t b = static_cast<uint16_t>(pop_ds());
    uint16_t a = static_cast<uint16_t>(pop_ds());
    push_ds(a < b ? -1 : 0);
}

void Forth::word_u_greater() {
    uint16_t b = static_cast<uint16_t>(pop_ds());
    uint16_t a = static_cast<uint16_t>(pop_ds());
    push_ds(a > b ? -1 : 0);
}

void Forth::word_u_less_equal() {
    uint16_t b = static_cast<uint16_t>(pop_ds());
    uint16_t a = static_cast<uint16_t>(pop_ds());
    push_ds(a <= b ? -1 : 0);
}

void Forth::word_u_greater_equal() {
    uint16_t b = static_cast<uint16_t>(pop_ds());
    uint16_t a = static_cast<uint16_t>(pop_ds());
    push_ds(a >= b ? -1 : 0);
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
    tib = line;
    in_ptr = 0;
    interpret_loop();
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

Word* Forth::define_word(const std::string& name, WordType type, XTCode code, const std::vector<Cell>& body, bool immediate) {
    std::string upper_name;
    for (char c : name) {
        upper_name += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    }
    int idx = static_cast<int>(words.size());
    Word w{upper_name, immediate, type, std::move(code), body, dp};
    words.push_back(std::move(w));
    words_by_name[upper_name] = idx;
    latest = idx;
    return &words.back();
}

Word* Forth::define_primitive(const std::string& name, XTCode code) {
    return define_word(name, WordType::Primitive, std::move(code), {}, false);
}

Word* Forth::define_immediate(const std::string& name, XTCode code) {
    return define_word(name, WordType::Primitive, std::move(code), {}, true);
}

Word* Forth::lookup(const std::string& name) {
    std::string upper_name;
    for (char c : name) {
        upper_name += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    }
    auto it = words_by_name.find(upper_name);
    if (it == words_by_name.end()) {
        return nullptr;
    }
    return &words[it->second];
}

int Forth::find_xt(const std::string& name) const {
    std::string upper_name;
    for (char c : name) {
        upper_name += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    }
    auto it = words_by_name.find(upper_name);
    if (it == words_by_name.end()) {
        return -1;
    }
    return it->second;
}

Word* Forth::latest_word() {
    if (latest < 0 || static_cast<size_t>(latest) >= words.size()) {
        return nullptr;
    }
    return &words[latest];
}

bool Forth::parse_word(std::string& word_out) {
    while (in_ptr < tib.size() && (tib[in_ptr] == ' ' || tib[in_ptr] == '\t' || tib[in_ptr] == '\r' || tib[in_ptr] == '\n')) {
        in_ptr++;
    }
    if (in_ptr >= tib.size()) {
        return false;
    }
    size_t start = in_ptr;
    while (in_ptr < tib.size() && tib[in_ptr] != ' ' && tib[in_ptr] != '\t' && tib[in_ptr] != '\r' && tib[in_ptr] != '\n') {
        in_ptr++;
    }
    word_out = tib.substr(start, in_ptr - start);
    return true;
}

void Forth::word_find() {
    Cell addr = pop_ds();
    uint32_t uaddr = static_cast<uint16_t>(addr);
    uint8_t len = c_fetch(uaddr);
    std::string name;
    for (uint8_t i = 0; i < len; ++i) {
        char ch = static_cast<char>(c_fetch(uaddr + 1 + i));
        name += static_cast<char>(std::toupper(static_cast<unsigned char>(ch)));
    }
    Word* w = lookup(name);
    if (!w) {
        push_ds(addr);
        push_ds(0);
        return;
    }
    int xt = find_xt(name);
    push_ds(static_cast<Cell>(xt));
    push_ds(w->immediate ? 1 : -1);
}

void Forth::word_tick() {
    std::string token;
    if (!parse_word(token)) {
        throw std::runtime_error("WORD NAME EXPECTED");
    }
    std::string upper_name;
    for (char c : token) {
        upper_name += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    }
    Word* w = lookup(upper_name);
    if (!w) {
        throw std::runtime_error("WORD NOT FOUND: " + token);
    }
    int xt = find_xt(upper_name);
    push_ds(static_cast<Cell>(xt));
}

void Forth::word_to_body() {
    Cell xt = pop_ds();
    if (xt >= 0 && static_cast<size_t>(xt) < words.size() && words[xt].pfa != 0) {
        push_ds(static_cast<Cell>(words[xt].pfa));
    } else {
        push_ds(static_cast<Cell>(xt + 2));
    }
}

Cell Forth::here() const {
    return fetch_cell(1002);
}

void Forth::allot(int n) {
    Cell cur = here();
    store_cell(1002, static_cast<Cell>(cur + n));
    dp = here();
}

void Forth::word_dp() {
    push_ds(1002);
}

void Forth::word_here() {
    push_ds(here());
}

void Forth::word_pad() {
    push_ds(static_cast<Cell>(here() + 68));
}

void Forth::word_allot() {
    Cell n = pop_ds();
    allot(n);
}

void Forth::word_comma() {
    Cell n = pop_ds();
    store_cell(static_cast<uint16_t>(here()), n);
    allot(2);
}

void Forth::word_c_comma() {
    Cell c = pop_ds();
    c_store(static_cast<uint16_t>(here()), static_cast<uint8_t>(c & 0xFF));
    allot(1);
}

void Forth::word_execute() {
    Cell xt = pop_ds();
    execute_word(xt);
}

void Forth::execute_word(int xt) {
    if (xt < 0 || static_cast<size_t>(xt) >= words.size()) {
        throw std::runtime_error("INVALID EXECUTION TOKEN: " + std::to_string(xt));
    }
    const Word& w = words[xt];
    switch (w.type) {
    case WordType::Primitive:
        if (w.code) {
            w.code(this);
        }
        break;
    case WordType::Constant:
        if (!w.body.empty()) {
            push_ds(w.body[0]);
        }
        break;
    case WordType::Variable:
    case WordType::Create:
        push_ds(static_cast<Cell>(w.pfa));
        break;
    case WordType::Colon: {
        std::vector<Cell> saved_body = current_body;
        int saved_ip = ip;
        current_body = w.body;
        ip = 0;
        try {
            while (ip < static_cast<int>(current_body.size())) {
                check_interrupt();
                int nxt = current_body[ip++];
                execute_word(nxt);
            }
        } catch (...) {
            current_body = saved_body;
            ip = saved_ip;
            throw;
        }
        current_body = saved_body;
        ip = saved_ip;
        break;
    }
    }
}

void Forth::word_create() {
    std::string token;
    if (!parse_word(token)) {
        throw std::runtime_error("WORD NAME EXPECTED");
    }
    Word* w = define_word(token, WordType::Create, nullptr, {}, false);
    w->pfa = here();
}

void Forth::word_constant() {
    Cell val = pop_ds();
    std::string token;
    if (!parse_word(token)) {
        throw std::runtime_error("WORD NAME EXPECTED");
    }
    define_word(token, WordType::Constant, nullptr, {val}, false);
}

void Forth::word_variable() {
    std::string token;
    if (!parse_word(token)) {
        throw std::runtime_error("WORD NAME EXPECTED");
    }
    Word* w = define_word(token, WordType::Variable, nullptr, {}, false);
    w->pfa = here();
    store_cell(static_cast<uint16_t>(here()), 0);
    allot(2);
}

int Forth::get_base() const {
    return fetch_cell(1004);
}

void Forth::set_base(int b) {
    store_cell(1004, static_cast<Cell>(b));
}

bool Forth::parse_number(const std::string& token, Cell& out_val) const {
    if (token.empty()) {
        return false;
    }
    int cur_base = get_base();
    if (cur_base <= 0) {
        cur_base = 10;
    }
    bool neg = false;
    size_t start = 0;
    if (token[0] == '-') {
        neg = true;
        start = 1;
    } else if (token[0] == '+') {
        start = 1;
    }
    if (start >= token.size()) {
        return false;
    }
    int parsed_base = cur_base;
    if (token[start] == '$') {
        parsed_base = 16;
        start++;
    } else if (token[start] == '%') {
        parsed_base = 2;
        start++;
    } else if (token[start] == '#') {
        start++;
        if (start < token.size() && token[start] == '#') {
            start++;
        }
    }
    if (start >= token.size()) {
        return false;
    }
    uint32_t val = 0;
    for (size_t i = start; i < token.size(); ++i) {
        char c = token[i];
        int digit = -1;
        if (c >= '0' && c <= '9') {
            digit = c - '0';
        } else if (c >= 'A' && c <= 'F') {
            digit = c - 'A' + 10;
        } else if (c >= 'a' && c <= 'f') {
            digit = c - 'a' + 10;
        } else {
            return false;
        }
        if (digit >= parsed_base) {
            return false;
        }
        val = val * parsed_base + digit;
    }
    Cell result = static_cast<Cell>(val & 0xFFFF);
    out_val = neg ? static_cast<Cell>(-result) : result;
    return true;
}

void Forth::word_word() {
    Cell delim_cell = pop_ds();
    char delim = static_cast<char>(delim_cell & 0xFF);
    while (in_ptr < tib.size() && tib[in_ptr] == delim) {
        in_ptr++;
    }
    uint32_t dest = static_cast<uint16_t>(here());
    if (in_ptr >= tib.size()) {
        c_store(dest, 0);
        push_ds(static_cast<Cell>(dest));
        return;
    }
    uint32_t start = dest + 1;
    uint8_t count = 0;
    while (in_ptr < tib.size() && tib[in_ptr] != delim && tib[in_ptr] != '\r' && tib[in_ptr] != '\n') {
        c_store(start + count, static_cast<uint8_t>(tib[in_ptr]));
        count++;
        in_ptr++;
    }
    c_store(dest, count);
    c_store(start + count, ' ');
    push_ds(static_cast<Cell>(dest));
}

void Forth::word_number() {
    Cell addr = pop_ds();
    uint32_t uaddr = static_cast<uint16_t>(addr);
    uint8_t len = c_fetch(uaddr);
    std::string token;
    for (uint8_t i = 0; i < len; ++i) {
        token += static_cast<char>(c_fetch(uaddr + 1 + i));
    }
    Cell num = 0;
    if (parse_number(token, num)) {
        push_ds(num);
        push_ds(-1); // success
    } else {
        push_ds(0);
        push_ds(0);  // failure
    }
}

void Forth::word_base() {
    push_ds(1004);
}

void Forth::interpret_loop() {
    while (in_ptr < tib.size()) {
        check_interrupt();
        std::string token;
        if (!parse_word(token)) {
            break;
        }
        std::string upper_name;
        for (char c : token) {
            upper_name += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
        }
        Word* w = lookup(upper_name);
        if (w) {
            int xt = find_xt(upper_name);
            if (state && !w->immediate) {
                compile_list.push_back(static_cast<Cell>(xt));
            } else {
                execute_word(xt);
            }
        } else {
            Cell num = 0;
            if (parse_number(token, num)) {
                if (state) {
                    int lit_xt = find_xt("LIT");
                    if (lit_xt < 0) {
                        define_primitive("LIT", [](Forth* f) { f->word_lit(); });
                        lit_xt = find_xt("LIT");
                    }
                    compile_list.push_back(static_cast<Cell>(lit_xt));
                    compile_list.push_back(num);
                } else {
                    push_ds(num);
                }
            } else {
                std::string err_token = token;
                abort();
                throw std::runtime_error("UNKNOWN WORD: " + err_token);
            }
        }
    }
}

void Forth::word_interpret() {
    interpret_loop();
}

void Forth::word_lit() {
    if (ip < static_cast<int>(current_body.size())) {
        push_ds(current_body[ip++]);
    }
}

void Forth::word_state() {
    push_ds(state ? -1 : 0);
}

void Forth::word_left_bracket() {
    state = false;
}

void Forth::word_right_bracket() {
    state = true;
}

void Forth::word_literal() {
    Cell val = pop_ds();
    int lit_xt = find_xt("LIT");
    if (lit_xt < 0) {
        define_primitive("LIT", [](Forth* f) { f->word_lit(); });
        lit_xt = find_xt("LIT");
    }
    compile_list.push_back(static_cast<Cell>(lit_xt));
    compile_list.push_back(val);
}

void Forth::word_compile() {
    std::string token;
    if (!parse_word(token)) {
        throw std::runtime_error("WORD NAME EXPECTED");
    }
    std::string upper_name;
    for (char c : token) {
        upper_name += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    }
    int xt = find_xt(upper_name);
    if (xt < 0) {
        throw std::runtime_error("WORD NOT FOUND: " + token);
    }
    compile_list.push_back(static_cast<Cell>(xt));
}

void Forth::word_bracket_compile() {
    std::string token;
    if (!parse_word(token)) {
        throw std::runtime_error("WORD NAME EXPECTED");
    }
    std::string upper_name;
    for (char c : token) {
        upper_name += static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    }
    int xt = find_xt(upper_name);
    if (xt < 0) {
        throw std::runtime_error("WORD NOT FOUND: " + token);
    }
    compile_list.push_back(static_cast<Cell>(xt));
}

void Forth::init_primitives() {
    define_primitive("DUP", [](Forth* f) { f->word_dup(); });
    define_primitive("DROP", [](Forth* f) { f->word_drop(); });
    define_primitive("SWAP", [](Forth* f) { f->word_swap(); });
    define_primitive("OVER", [](Forth* f) { f->word_over(); });
    define_primitive("?DUP", [](Forth* f) { f->word_qdup(); });
    define_primitive("ROT", [](Forth* f) { f->word_rot(); });
    define_primitive("-ROT", [](Forth* f) { f->word_nrot(); });
    define_primitive("NIP", [](Forth* f) { f->word_nip(); });
    define_primitive("TUCK", [](Forth* f) { f->word_tuck(); });
    define_primitive("2DUP", [](Forth* f) { f->word_2dup(); });
    define_primitive("2DROP", [](Forth* f) { f->word_2drop(); });
    define_primitive("2SWAP", [](Forth* f) { f->word_2swap(); });
    define_primitive("2OVER", [](Forth* f) { f->word_2over(); });

    define_primitive(">R", [](Forth* f) { f->word_to_r(); });
    define_primitive("R>", [](Forth* f) { f->word_r_from(); });
    define_primitive("R@", [](Forth* f) { f->word_r_fetch(); });

    define_primitive("@", [](Forth* f) { f->word_fetch(); });
    define_primitive("!", [](Forth* f) { f->word_store(); });
    define_primitive("C@", [](Forth* f) { f->word_c_fetch(); });
    define_primitive("C!", [](Forth* f) { f->word_c_store(); });
    define_primitive("+!", [](Forth* f) { f->word_plus_store(); });
    define_primitive("?", [](Forth* f) { f->word_question(); });

    define_primitive("FILL", [](Forth* f) { f->word_fill(); });
    define_primitive("ERASE", [](Forth* f) { f->word_erase(); });
    define_primitive("CMOVE", [](Forth* f) { f->word_cmove(); });
    define_primitive("CMOVE>", [](Forth* f) { f->word_cmover(); });

    define_primitive("+", [](Forth* f) { f->word_plus(); });
    define_primitive("-", [](Forth* f) { f->word_minus(); });
    define_primitive("*", [](Forth* f) { f->word_multiply(); });
    define_primitive("/MOD", [](Forth* f) { f->word_slash_mod(); });
    define_primitive("/", [](Forth* f) { f->word_slash(); });
    define_primitive("MOD", [](Forth* f) { f->word_mod(); });
    define_primitive("NEGATE", [](Forth* f) { f->word_negate(); });

    define_primitive("1+", [](Forth* f) { f->word_one_plus(); });
    define_primitive("1-", [](Forth* f) { f->word_one_minus(); });
    define_primitive("2+", [](Forth* f) { f->word_two_plus(); });
    define_primitive("2-", [](Forth* f) { f->word_two_minus(); });
    define_primitive("2*", [](Forth* f) { f->word_two_star(); });
    define_primitive("2/", [](Forth* f) { f->word_two_slash(); });
    define_primitive("ABS", [](Forth* f) { f->word_abs(); });
    define_primitive("MIN", [](Forth* f) { f->word_min(); });
    define_primitive("MAX", [](Forth* f) { f->word_max(); });

    define_primitive("AND", [](Forth* f) { f->word_and(); });
    define_primitive("OR", [](Forth* f) { f->word_or(); });
    define_primitive("XOR", [](Forth* f) { f->word_xor(); });
    define_primitive("INVERT", [](Forth* f) { f->word_invert(); });

    define_primitive("=", [](Forth* f) { f->word_equals(); });
    define_primitive("<>", [](Forth* f) { f->word_not_equals(); });
    define_primitive("<", [](Forth* f) { f->word_less(); });
    define_primitive(">", [](Forth* f) { f->word_greater(); });
    define_primitive("<=", [](Forth* f) { f->word_less_equal(); });
    define_primitive(">=", [](Forth* f) { f->word_greater_equal(); });
    define_primitive("0=", [](Forth* f) { f->word_zero_equals(); });
    define_primitive("0<>", [](Forth* f) { f->word_zero_not_equals(); });
    define_primitive("0<", [](Forth* f) { f->word_zero_less(); });
    define_primitive("0>", [](Forth* f) { f->word_zero_greater(); });
    define_primitive("U<", [](Forth* f) { f->word_u_less(); });
    define_primitive("U>", [](Forth* f) { f->word_u_greater(); });
    define_primitive("U<=", [](Forth* f) { f->word_u_less_equal(); });
    define_primitive("U>=", [](Forth* f) { f->word_u_greater_equal(); });

    define_primitive("KEY", [](Forth* f) { f->key(); });
    define_primitive("EMIT", [](Forth* f) { f->emit(); });
    define_primitive("CR", [](Forth* f) { f->cr(); });
    define_primitive("SPACE", [](Forth* f) { f->space(); });
    define_primitive("SPACES", [](Forth* f) { f->spaces(); });
    define_primitive("TYPE", [](Forth* f) { f->type(); });
    define_primitive("BYE", [](Forth* f) { f->bye(); });
    define_primitive("EXPECT", [](Forth* f) { f->expect(); });
    define_primitive("SPAN", [](Forth* f) { f->span(); });
    define_primitive("QUIT", [](Forth* f) { f->quit(); });
    define_primitive("ABORT", [](Forth* f) { f->abort(); });

    define_primitive("FIND", [](Forth* f) { f->word_find(); });
    define_primitive("'", [](Forth* f) { f->word_tick(); });
    define_primitive(">BODY", [](Forth* f) { f->word_to_body(); });

    define_primitive("DP", [](Forth* f) { f->word_dp(); });
    define_primitive("HERE", [](Forth* f) { f->word_here(); });
    define_primitive("PAD", [](Forth* f) { f->word_pad(); });
    define_primitive("ALLOT", [](Forth* f) { f->word_allot(); });
    define_primitive(",", [](Forth* f) { f->word_comma(); });
    define_primitive("C,", [](Forth* f) { f->word_c_comma(); });
    define_primitive("EXECUTE", [](Forth* f) { f->word_execute(); });

    define_primitive("CREATE", [](Forth* f) { f->word_create(); });
    define_primitive("CONSTANT", [](Forth* f) { f->word_constant(); });
    define_primitive("VARIABLE", [](Forth* f) { f->word_variable(); });

    define_primitive("WORD", [](Forth* f) { f->word_word(); });
    define_primitive("NUMBER", [](Forth* f) { f->word_number(); });
    define_primitive("BASE", [](Forth* f) { f->word_base(); });
    define_primitive("INTERPRET", [](Forth* f) { f->word_interpret(); });
    define_primitive("LIT", [](Forth* f) { f->word_lit(); });

    define_primitive("STATE", [](Forth* f) { f->word_state(); });
    define_immediate("[", [](Forth* f) { f->word_left_bracket(); });
    define_primitive("]", [](Forth* f) { f->word_right_bracket(); });
    define_immediate("LITERAL", [](Forth* f) { f->word_literal(); });
    define_primitive("COMPILE", [](Forth* f) { f->word_compile(); });
    define_immediate("[COMPILE]", [](Forth* f) { f->word_bracket_compile(); });
}

} // namespace pforth
