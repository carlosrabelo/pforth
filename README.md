# PForth

Deterministic 16-bit Forth interpreter and compiler.

## Prerequisites

- **Make** — GNU Make 4.0+
- **GCC / Clang** — C++20 for the `cpp/` component
- **Go 1.24+** — for the `go/` component

## Installation

```bash
make setup
make build
```

## Layout

- `cpp/` — C++20 port
- `go/` — Go port
- `pforth/` — shared Forth libraries
- `demos/` — demonstration programs

## Development

```bash
make build
make test
make quality
make clean
```

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
