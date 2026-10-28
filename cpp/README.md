# PForth (C++)

C++20 implementation of the PForth 16-bit deterministic Forth interpreter and compiler.

## Highlights

- Implemented in modern C++20 conforming to strict compiler warnings (-Wall -Wextra -Wpedantic)
- Flat 64KB byte-addressable linear RAM memory model with bounds checking
- Dual stacks architecture (Data Stack and Return Stack) with underflow guards
- Native execution engine supporting colon definitions, branching, and counted loops
- Test automation powered by Catch2 single-header testing framework

## Prerequisites

- **GCC / Clang** — C++20 compiler (`g++` 11+ or `clang++` 13+)
- **Make** — GNU Make 4.0+
- **Curl** — Required by `make setup` to download Catch2

## Installation

```bash
make setup
make build
```

## Usage

```bash
# Start REPL
./bin/pforth

# Run demo script
./bin/pforth ../demos/sieve.fs
```

## Development

```bash
make setup           # Download Catch2 test framework
make build           # Compile binary to bin/pforth
make test            # Compile and run Catch2 test suite
make quality         # Run formatting and static checks
make clean           # Remove build artifacts
```

## License

This project is licensed under the MIT License — see [LICENSE](../LICENSE) for details.
