\ pForth core library
\ composite words defined in Forth from kernel primitives

\ Stack (depends only on kernel primitives)
: NIP    SWAP DROP ;
: TUCK   SWAP OVER ;
: ROT    >R SWAP R> SWAP ;
: -ROT   SWAP >R SWAP R> ;
: 2DUP   OVER OVER ;
: 2DROP  DROP DROP ;

\ Division (depends on NIP, DROP from stack section)
: /      /MOD NIP ;
: MOD    /MOD DROP ;

\ Comparison basics (depends on kernel primitives)
: 0=     0 = ;
: <>     = 0= ;
: 0<>    0 <> ;
: 0<     0 < ;
: 0>     0 > ;
: <=     OVER OVER < ROT ROT = OR ;
: >=     OVER OVER > ROT ROT = OR ;

\ Arithmetic compounds (depends on kernel primitives)
: 1+     1 + ;
: 1-     1 - ;
: 2+     2 + ;
: 2-     2 - ;
: 2*     2 * ;
: 2/     2 / ;
: NEGATE 0 SWAP - ;
: +!     TUCK @ + SWAP ! ;

\ Extended arithmetic (depends on comparison and 2DUP above)
: ABS    DUP 0< IF NEGATE THEN ;
: MIN    2DUP < IF DROP ELSE SWAP DROP THEN ;
: MAX    2DUP > IF DROP ELSE SWAP DROP THEN ;

\ I/O (depends only on kernel primitives)
: SPACE  32 EMIT ;
: SPACES 0 MAX 0 DO SPACE LOOP ;
: ?      @ . ;
: PAGE   12 EMIT ;

\ Misc
: ERASE  0 FILL ;
