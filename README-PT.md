# PForth

Interpretador e compilador Forth determinístico de 16 bits implementado em linguagens modernas de programação.

## Destaques

- Execução simples e determinística de Forth modelada segundo sistemas clássicos Forth de 16 bits
- Modelo de memória de 64KB endereçável por byte com pilhas separadas de dados (DS) e retorno (RS)
- REPL interativo e execução de scripts em lote não interativa
- Palavras essenciais de controle de fluxo incluindo IF, ELSE, THEN, BEGIN, UNTIL, WHILE, REPEAT e DO/LOOP
- Primitivas aritméticas inteiras com semântica de divisão e módulo de 16 bits (/MOD, /, MOD)
- Arquitetura de dicionário em duas camadas combinando primitivas nativas do núcleo com definições Forth
- Implementações consistentes em múltiplas linguagens compartilhando a mesma semântica de sintaxe e suítes de testes

## Pré-requisitos

- **GCC / Clang** — compilador C++20 necessário para o componente C++ (`g++` 11+ ou `clang++` 13+)
- **Make** — GNU Make 4.0+ para automação de compilação

## Instalação

### Compilar a partir do Código-Fonte

Clone o repositório e compile os componentes:

```bash
git clone https://github.com/carlosrabelo/pforth.git
cd pforth
make build
```

## Uso

### REPL Interativo

Inicie o interpretador sem argumentos:

```bash
./cpp/bin/pforth
```

Sessão de exemplo:

```forth
1 2 + .
3 ok

: SQUARE DUP * ;
ok
5 SQUARE .
25 ok
```

### Execução de Script em Lote

Execute um arquivo fonte Forth diretamente:

```bash
./cpp/bin/pforth program.fs
```

## Estrutura do Projeto

```
cpp/                 # C++20 implementation and Catch2 test suite
go/                  # Go implementation and test suite
pascal/              # Free Pascal implementation and test suite
rust/                # Rust implementation and test suite
Makefile             # Root orchestrator Makefile
LICENSE              # MIT License
```

## Desenvolvimento

```bash
make build           # Compilar todos os componentes
make test            # Executar testes em todos os componentes
make quality         # Executar verificações de formatação e análise estática
make clean           # Remover artefatos de compilação
```

## Licença

Este projeto está licenciado sob a Licença MIT — consulte [LICENSE](LICENSE) para obter detalhes.
