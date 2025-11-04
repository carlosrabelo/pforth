# PForth (C++)

C++20 implementation of the PForth 16-bit deterministic Forth interpreter and compiler.

## Prerequisites

- **GCC / Clang** — C++20 compiler (`g++` 11+ or `clang++` 13+)
- **Make** — GNU Make 4.0+
- **Curl** — Required by `make setup` to download Catch2

## Installation

```bash
make setup
make build
```

## Development

```bash
make setup           # Download Catch2 test framework
make build           # Compile binary to bin/pforth
make test            # Compile and run tests
make quality         # Formatting checks when clang-format is available
make clean           # Remove build artifacts
```

## License

This project is licensed under the MIT License. See [LICENSE](../LICENSE).
