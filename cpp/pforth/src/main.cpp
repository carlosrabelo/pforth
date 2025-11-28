#include "cli.hpp"
#include <iostream>

int main(int argc, char* argv[]) {
    auto args = pforth::parse_arguments(argc, argv);
    return pforth::run_cli(args, std::cin, std::cout, std::cerr);
}
