#pragma once

#include <iostream>
#include <string>
#include <vector>

namespace pforth {

struct CliArgs {
    bool interactive{true};
    std::vector<std::string> file_paths;
};

CliArgs parse_arguments(int argc, char* argv[]);

int run_cli(const CliArgs& args, std::istream& in, std::ostream& out, std::ostream& err);

} // namespace pforth
