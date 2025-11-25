VARIABLE SRC
VARIABLE DEST
VARIABLE AUX
: HANOI ( n src dest aux -- )
  AUX ! DEST ! SRC !
  DUP IF
    SRC @ DEST @ AUX @
    >R >R >R
    DUP 1- SRC @ AUX @ DEST @ RECURSE
    R> SRC ! R> DEST ! R> AUX !
    ." Move " SRC @ . ." -> " DEST @ . CR
    1- AUX @ DEST @ SRC @ RECURSE
  ELSE
    DROP
  THEN
;
3 1 3 2 HANOI
