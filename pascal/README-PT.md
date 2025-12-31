# PForth (Free Pascal)

Implementação em Free Pascal do interpretador e compilador Forth determinístico de 16 bits PForth.

## Destaques

- Implementado em Free Pascal moderno (modo Object Pascal `-Mobjfpc -Sh`)
- Modelo de memória RAM linear de 64KB endereçável por byte com verificação de limites
- Arquitetura de pilhas duplas (Pilha de Dados e Pilha de Retorno) com proteção contra underflow
- Motor de execução nativo com suporte a definições com dois-pontos, desvios e laços contados
- Executor de testes leve integrado para verificação unitária e de integração

## Pré-requisitos

- **Free Pascal Compiler** — `fpc` 3.2.0+
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
make test            # Compilar e executar executor de testes
make quality         # Executar verificações de qualidade
make clean           # Remover artefatos de compilação
```

## Licença

Este projeto é licenciado sob a Licença MIT — consulte [LICENSE](../LICENSE) para mais detalhes.
