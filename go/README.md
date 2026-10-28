# PForth (Go)

Go 1.23+ implementation of the PForth 16-bit deterministic Forth interpreter and compiler.

## Highlights

- Implemented in clean, modern Go following standard Go project skeleton layout
- Native execution engine supporting both interactive REPL and batch script execution
- Integrated 64KB linear byte memory model with little-endian 16-bit cell access
- Dual-layer dictionary coupling native Go primitives with high-level Forth definitions
- Automatic library and demo resolution via #REQUIRE directive walking directory hierarchy

## Prerequisites

- **Go 1.23+** — [Download Go](https://go.dev/dl/)
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
make test            # Run unit test suite
make quality         # Format and vet code
make clean           # Remove build artifacts
```

## License

This project is licensed under the MIT License — see [LICENSE](../LICENSE) for details.
