# PForth (Rust)

Rust implementation of the PForth 16-bit deterministic Forth interpreter and compiler.

## Prerequisites

- **Rust** — stable `cargo` / `rustc` on `PATH`
- **Make** — GNU Make 4.0+

## Installation

```bash
make build
```

## Development

```bash
make build           # Compile and copy binary to bin/pforth
make test            # Run cargo test
make quality         # rustfmt check and clippy
make clean           # Remove target/ and bin/pforth
```

## License

This project is licensed under the MIT License — see [LICENSE](../LICENSE) for details.
