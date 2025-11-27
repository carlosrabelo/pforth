# PForth (C++)

Implementação C++20 do interpretador e compilador Forth determinístico de 16 bits PForth.

## Pré-requisitos

- **GCC / Clang** — compilador C++20 (`g++` 11+ ou `clang++` 13+)
- **Make** — GNU Make 4.0+
- **Curl** — necessário para `make setup` descarregar o Catch2

## Instalação

```bash
make setup
make build
```

## Desenvolvimento

```bash
make setup           # Descarregar o framework de testes Catch2
make build           # Compilar o binário em bin/pforth
make test            # Compilar e executar testes
make quality         # Verificações de formatação quando clang-format estiver disponível
make clean           # Remover artefactos de compilação
```

## Bibliotecas

`make test` define `PFORTH_LIB` para o diretório `pforth/` do repositório para o motor carregar `CORE` no arranque a frio.

## Licença

Este projeto está licenciado sob a Licença MIT. Ver [LICENSE](../LICENSE).
