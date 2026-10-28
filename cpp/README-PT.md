# PForth (C++)

Implementação em C++20 do interpretador e compilador Forth determinístico de 16 bits PForth.

## Destaques

- Implementado em C++20 moderno em conformidade com avisos rigorosos do compilador (-Wall -Wextra -Wpedantic)
- Modelo de memória RAM linear de 64KB endereçável por byte com verificação de limites
- Arquitetura de pilhas duplas (Pilha de Dados e Pilha de Retorno) com proteção contra underflow
- Motor de execução nativo com suporte a definições com dois-pontos, desvios e laços contados
- Automação de testes baseada no framework de teste em cabeçalho único Catch2

## Pré-requisitos

- **GCC / Clang** — Compilador C++20 (`g++` 11+ ou `clang++` 13+)
- **Make** — GNU Make 4.0+
- **Curl** — Necessário para `make setup` baixar o Catch2

## Instalação

```bash
make setup
make build
```

## Uso

```bash
# Iniciar REPL
./bin/pforth

# Executar script de demonstração
./bin/pforth ../demos/sieve.fs
```

## Desenvolvimento

```bash
make setup           # Baixar framework de testes Catch2
make build           # Compilar binário em bin/pforth
make test            # Compilar e executar suíte de testes Catch2
make quality         # Executar verificações de formatação e análise estática
make clean           # Remover artefatos de compilação
```

## Licença

Este projeto é licenciado sob a Licença MIT — consulte [LICENSE](../LICENSE) para mais detalhes.
