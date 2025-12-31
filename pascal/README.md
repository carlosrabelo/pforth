# PForth (Free Pascal)

Free Pascal implementation of the PForth 16-bit deterministic Forth interpreter and compiler.

## Highlights

- Implemented in modern Free Pascal (Object Pascal mode `-Mobjfpc -Sh`)
- Flat 64KB byte-addressable linear RAM memory model with bounds checking
- Dual stacks architecture (Data Stack and Return Stack) with underflow guards
- Native execution engine supporting colon definitions, branching, and counted loops
- Built-in lightweight test runner for unit and integration verification

## Prerequisites

- **Free Pascal Compiler** — `fpc` 3.2.0+
- **Make** — GNU Make 4.0+

## Installation

```bash
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
make build           # Compile binary to bin/pforth
make test            # Compile and run test runner
make quality         # Run quality checks
make clean           # Remove build artifacts
```

## License

This project is licensed under the MIT License — see [LICENSE](../LICENSE) for details.
