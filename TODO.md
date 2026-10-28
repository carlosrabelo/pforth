# pForth

## First milestone

Minimal Z80 bootstrap compiler and interactive outer interpreter loop running on z88dk-ticks.

## Z80 Foundation

- [x] CPU memory layout configuration
- [x] Inner interpreter loop (NEXT, DOCOL, SEMI)

## Z80 Minimal TTY & Input Processing

- [x] KEY
- [x] EMIT
- [x] WORD
- [x] NUMBER
- [x] FIND
- [x] EXECUTE

## Z80 Compiler & Outer Loop

- [x] STATE
- [x] LIT
- [x] INTERPRET
- [x] QUIT
- [x] COLD
- [x] [
- [x] ]
- [x] CREATE
- [x] ,
- [x] IMMEDIATE
- [x] :
- [x] ;

## Test Automation

- [x] Z80 assembly test harness
- [x] Python test runner script
- [x] Makefile test target integration

## Second milestone

Standard pFORTH dictionary expansion and validation.

## Z80 Dictionary Expansion

### Stack Manipulation

- [x] DUP
- [x] DROP
- [x] SWAP
- [x] OVER
- [x] ROT
- [x] ?DUP
- [x] >R
- [x] R>
- [x] R@

### Memory & Register Access

- [x] @
- [x] !
- [x] C@
- [x] C!
- [x] +!

### Logic & Math

- [x] +
- [x] -
- [x] AND
- [x] OR
- [x] XOR
- [x] 0=
- [x] 0<
- [x] U<
- [x] =
- [x] <
- [x] >
- [x] *
- [x] /MOD
- [x] /
- [x] MOD

### Compiler & Variables

- [x] HERE
- [x] PAD
- [x] ALLOT
- [x] C,
- [x] CONSTANT
- [x] VARIABLE
- [x] [COMPILE]
- [x] COMPILE
- [x] LITERAL

### Control Flow

- [x] IF
- [x] ELSE
- [x] THEN
- [x] BEGIN
- [x] UNTIL
- [x] WHILE
- [x] REPEAT
- [x] DO
- [x] LOOP
- [x] +LOOP
- [x] I
- [x] LEAVE

### Formatting & TTY

- [x] CR
- [x] SPACE
- [x] SPACES
- [x] TYPE
- [x] EXPECT

---

## Third milestone

Forth demonstration applications.

## Demos

- [x] Sieve of Eratosthenes benchmark
- [x] Block-based line editor
- [x] Mini text adventure game
- [x] WORDS and DUMP system tools
