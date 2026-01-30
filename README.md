# PForth

Deterministic 16-bit Forth interpreter and compiler.

## Prerequisites

- **Make** — GNU Make 4.0+
- **GCC / Clang** — C++20 for the `cpp/` component
- **Go 1.24+** — for the `go/` component
- **Free Pascal** — `fpc` for the `pascal/` component
- **Rust** — stable `cargo` / `rustc` for the `rust/` component

## Installation

```bash
make setup
make build
```

## Layout

- `cpp/` — C++20 port
- `go/` — Go port
- `pascal/` — Free Pascal port
- `rust/` — Rust port
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
