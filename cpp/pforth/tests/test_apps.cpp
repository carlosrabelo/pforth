#include "catch.hpp"
#include "forth.hpp"
#include <cstdlib>
#include <unistd.h>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <sstream>
#include <string>

// Shared Forth libraries under pforth/: editor.fs and adventure.fs.

namespace {

std::string repeat(char c, size_t n) { return std::string(n, c); }

std::string read_all(const std::filesystem::path& p) {
    std::ifstream in(p, std::ios::binary);
    return std::string(std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>());
}

std::string play_script(const std::string& script, bool* running, bool* has_module) {
    std::istringstream in(script);
    std::ostringstream out;
    pforth::Forth f(&in, &out);
    REQUIRE_THROWS_WITH(f.interpret_line("PLAY"), "UNKNOWN WORD: PLAY");
    f.interpret_line("#REQUIRE ADVENTURE");
    f.interpret_line("PLAY");
    if (running != nullptr) {
        *running = f.running;
    }
    if (has_module != nullptr) {
        *has_module = f.loaded_modules.count("ADVENTURE") > 0;
    }
    return out.str();
}

} // namespace

TEST_CASE("Block editor library", "[apps][editor]") {
    std::istringstream in;
    std::ostringstream out;
    pforth::Forth f(&in, &out);

    REQUIRE_THROWS_WITH(f.interpret_line("LIST"), "UNKNOWN WORD: LIST");
    f.interpret_line("#REQUIRE EDITOR");
    REQUIRE(f.loaded_modules.count("EDITOR") > 0);

    REQUIRE_THROWS_WITH(f.interpret_line("FLUSH"), "EDITOR FILE");
    REQUIRE_THROWS_WITH(f.interpret_line("OPEN"), "FILE NAME EXPECTED");
    REQUIRE_THROWS_WITH(f.interpret_line("OPEN /no/such/dir/block.txt"),
                        "CANNOT OPEN: /no/such/dir/block.txt");

    std::error_code ec;
    const std::filesystem::path path =
        std::filesystem::temp_directory_path(ec) / ("pforth_editor_cpp_" + std::to_string(::getpid()) + ".blk");
    {
        std::ofstream w(path, std::ios::binary);
        w << "first line";
    }
    const std::string name = path.string();

    f.interpret_line("OPEN " + name);
    out.str("");
    f.interpret_line("LIST");
    std::string expected = "0: first line\n";
    for (int line = 1; line < 16; ++line) {
        expected += std::to_string(line) + ": \n";
    }
    REQUIRE(out.str() == expected);

    f.interpret_line("0 P hello world");
    f.interpret_line("15 P last");
    f.interpret_line("3 P");
    REQUIRE_THROWS_WITH(f.interpret_line("16 P x"), "EDITOR LINE");
    REQUIRE_THROWS_WITH(f.interpret_line("-1 P x"), "EDITOR LINE");
    out.str("");
    f.interpret_line("LIST");
    const std::string listing = out.str();
    REQUIRE(listing.rfind("0: hello world\n1: \n2: \n3: \n", 0) == 0);
    const std::string tail = "15: last\n";
    REQUIRE(listing.size() >= tail.size());
    REQUIRE(listing.compare(listing.size() - tail.size(), tail.size(), tail) == 0);

    f.interpret_line("1 P " + repeat('x', 70));
    f.interpret_line("FLUSH");
    const std::string saved = read_all(path);
    REQUIRE(saved.size() == 1024);
    REQUIRE(saved.substr(0, 64) == "hello world" + repeat(' ', 53));
    REQUIRE(saved.substr(64, 64) == repeat('x', 64));
    REQUIRE(saved.substr(128, 4) == "    ");
    REQUIRE(saved.substr(15 * 64, 4) == "last");

    f.interpret_line("OPEN " + name);
    out.str("");
    f.interpret_line("LIST");
    REQUIRE(out.str().rfind("0: hello world\n1: xxxx", 0) == 0);

    std::filesystem::remove(path, ec);
}

