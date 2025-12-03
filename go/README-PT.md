# PForth (Go)

Implementação em Go 1.23+ do interpretador e compilador Forth determinístico de 16 bits PForth.

## Destaques

- Implementado em Go limpo e moderno seguindo o padrão de projeto go-skeleton
- Motor de execução nativo com suporte a REPL interativo e execução de scripts em lote
- Modelo de memória linear de 64KB com acesso a células de 16 bits em little-endian
- Dicionário em duas camadas associando primitivas Go nativas com definições Forth
- Resolução automática de bibliotecas e demonstrações via diretiva #REQUIRE percorrendo diretórios

## Pré-requisitos

- **Go 1.23+** — [Download Go](https://go.dev/dl/)
- **Make** — GNU Make 4.0+

## Instalação

```bash
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
make build           # Compilar binário em bin/pforth
make test            # Executar suíte de testes unitários
make quality         # Formatar e analisar código
make clean           # Remover artefatos de compilação
```

## Licença

Este projeto é licenciado sob a Licença MIT — consulte [LICENSE](../LICENSE) para mais detalhes.
