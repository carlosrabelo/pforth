#include "cli.hpp"
#include "forth.hpp"
#include <fstream>

namespace pforth {

CliArgs parse_arguments(int argc, char* argv[]) {
    CliArgs args;
    for (int i = 1; i < argc; ++i) {
        args.file_paths.emplace_back(argv[i]);
    }
    args.interactive = (argc == 1);
    return args;
}

int run_cli(const CliArgs& args, std::istream& in, std::ostream& out, std::ostream& err) {
    Forth f(&in, &out);

    for (const auto& path : args.file_paths) {
        try {
            f.load_file(path);
        } catch (const std::exception& e) {
            err << e.what() << "\n";
            return 1;
        }
    }

    if (args.interactive) {
        f.quit();
    }
    return 0;
}

} // namespace pforth
