#define CATCH_CONFIG_MAIN
#include "catch.hpp"
#include "cli.hpp"
#include <sstream>

TEST_CASE("CLI argument parser - default interactive mode", "[cli]") {
    char prog[] = "pforth";
    char* argv[] = {prog};
    int argc = 1;

    auto args = pforth::parse_arguments(argc, argv);
    REQUIRE(args.interactive == true);
    REQUIRE(args.file_paths.empty());
}

TEST_CASE("CLI argument parser - single script file", "[cli]") {
    char prog[] = "pforth";
    char file1[] = "demo.fs";
    char* argv[] = {prog, file1};
    int argc = 2;

    auto args = pforth::parse_arguments(argc, argv);
    REQUIRE(args.interactive == false);
    REQUIRE(args.file_paths.size() == 1);
    REQUIRE(args.file_paths[0] == "demo.fs");
}

TEST_CASE("CLI argument parser - multiple script files", "[cli]") {
    char prog[] = "pforth";
    char file1[] = "libs/core.fs";
    char file2[] = "demos/sieve.fs";
    char* argv[] = {prog, file1, file2};
    int argc = 3;

    auto args = pforth::parse_arguments(argc, argv);
    REQUIRE(args.interactive == false);
    REQUIRE(args.file_paths.size() == 2);
    REQUIRE(args.file_paths[0] == "libs/core.fs");
    REQUIRE(args.file_paths[1] == "demos/sieve.fs");
}

TEST_CASE("CLI runner - missing file returns error", "[cli]") {
    pforth::CliArgs args;
    args.interactive = false;
    args.file_paths.push_back("non_existent_file_xyz_123.fs");

    std::stringstream in;
    std::stringstream out;
    std::stringstream err;

    int code = pforth::run_cli(args, in, out, err);
    REQUIRE(code == 1);
    REQUIRE(err.str().find("CANNOT OPEN") != std::string::npos);
}
