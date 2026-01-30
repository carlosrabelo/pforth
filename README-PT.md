# PForth

Interpretador e compilador Forth determinístico de 16 bits.

## Pré-requisitos

- **Make** — GNU Make 4.0+
- **GCC / Clang** — C++20 para o componente `cpp/`
- **Go 1.24+** — para o componente `go/`
- **Free Pascal** — `fpc` para o componente `pascal/`
- **Rust** — `cargo` / `rustc` estável para o componente `rust/`

## Instalação

```bash
make setup
make build
```

## Estrutura

- `cpp/` — porta C++20
- `go/` — porta Go
- `pascal/` — porta Free Pascal
- `rust/` — porta Rust
- `pforth/` — bibliotecas Forth partilhadas
- `demos/` — programas de demonstração

## Desenvolvimento

```bash
make build
make test
make quality
make clean
```

## Licença

Este projeto está licenciado sob a Licença MIT. Ver [LICENSE](LICENSE).
