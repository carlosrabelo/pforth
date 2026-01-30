# PForth (Rust)

Implementação em Rust do interpretador e compilador Forth determinístico de 16 bits PForth.

## Pré-requisitos

- **Rust** — `cargo` / `rustc` estável no `PATH`
- **Make** — GNU Make 4.0+

## Instalação

```bash
make build
```

## Desenvolvimento

```bash
make build           # Compila e copia o binário para bin/pforth
make test            # Executa cargo test
make quality         # rustfmt check e clippy
make clean           # Remove target/ e bin/pforth
```

## Licença

Este projeto está licenciado sob a Licença MIT — veja [LICENSE](../LICENSE) para detalhes.
