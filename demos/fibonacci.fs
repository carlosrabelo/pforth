: FIB ( n -- fib[n] )
  0 1 ROT 0 DO
    TUCK +
  LOOP DROP
;
10 FIB . CR
