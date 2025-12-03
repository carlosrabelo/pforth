# PForth

Interpretador e compilador Forth determinístico de 16 bits.

## Pré-requisitos

- **Make** — GNU Make 4.0+
- **GCC / Clang** — C++20 para o componente `cpp/`

## Instalação

```bash
make setup
make build
```

## Estrutura

- `cpp/`
- `go/` — porta Go — porta C++20
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
