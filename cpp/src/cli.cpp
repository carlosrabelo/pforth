#include "cli.hpp"
#include <fstream>

namespace pforth {

CliArgs parse_arguments(int argc, char* argv[]) {
    CliArgs args;
    for (int i = 1; i < argc; ++i) {
        args.file_paths.emplace_back(argv[i]);
    }
    // If files are given, interactive is still enabled if stdin is open or continued
    args.interactive = (argc == 1);
    return args;
}

int run_cli(const CliArgs& args, std::istream& in, std::ostream& out, std::ostream& err) {
    (void)in;
    (void)out;
    for (const auto& path : args.file_paths) {
        std::ifstream file(path);
        if (!file.is_open()) {
            err << "CANNOT OPEN: " << path << "\n";
            return 1;
        }
    }
    return 0;
}

} // namespace pforth
