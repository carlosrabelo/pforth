# PForth

Deterministic 16-bit Forth interpreter and compiler implemented across modern programming languages.

## Highlights

- Simple, deterministic Forth execution modeled after classic 16-bit Forth systems
- Byte-addressable 64KB memory model with separate data (DS) and return (RS) stacks
- Interactive REPL and non-interactive batch script execution
- Essential control flow words including IF, ELSE, THEN, BEGIN, UNTIL, WHILE, REPEAT, and DO/LOOP
- Integer arithmetic primitives with 16-bit division and modulo semantics (/MOD, /, MOD)
- Dual-layer dictionary architecture combining native kernel primitives with high-level Forth definitions
- Consistent, multi-language implementations sharing identical syntax semantics and test suites

## Prerequisites

- **GCC / Clang** — C++20 compiler required for the C++ component (`g++` 11+ or `clang++` 13+)
- **Make** — GNU Make 4.0+ for build automation

## Installation

### Build from Source

Clone the repository and compile the components:

```bash
git clone https://github.com/carlosrabelo/pforth.git
cd pforth
make build
```

## Usage

### Interactive REPL

Start the interpreter without arguments:

```bash
./cpp/bin/pforth
```

Example session:

```forth
1 2 + .
3 ok

: SQUARE DUP * ;
ok
5 SQUARE .
25 ok
```

### Batch Script Execution

Execute a Forth source file directly:

```bash
./cpp/bin/pforth program.fs
```

## Project Layout

```
cpp/                 # C++20 implementation and Catch2 test suite
go/                  # Go implementation and test suite
pascal/              # Free Pascal implementation and test suite
rust/                # Rust implementation and test suite
Makefile             # Root orchestrator Makefile
LICENSE              # MIT License
```

## Development

```bash
make build           # Compile all components
make test            # Run tests across all components
make quality         # Run formatting and static analysis checks
make clean           # Remove build artifacts
```

## License

This project is licensed under the MIT License — see [LICENSE](LICENSE) for details.
