\ Text adventure (pure Forth). PLAY reads lines until RUNNING is false (EOF).

VARIABLE ROOM
CREATE CMD 84 ALLOT
VARIABLE CSTART
VARIABLE CLEN

: UPCASE ( c -- c )
  DUP 97 < IF EXIT THEN
  DUP 122 > IF EXIT THEN
  32 - ;

: PREPARE-CMD ( -- )
  SPAN @ 0 DO
    CMD I + C@ UPCASE CMD I + C!
  LOOP
  0 CSTART !
  SPAN @ 0 DO
    CMD I + C@ 32 <> IF I CSTART ! LEAVE THEN
  LOOP
  SPAN @ CLEN !
  BEGIN
    CLEN @ CSTART @ > IF
      CMD CLEN @ 1- + C@ 32 =
    ELSE 0 THEN
  WHILE
    -1 CLEN +!
  REPEAT
  CLEN @ CSTART @ - CLEN !
;

: CMD= ( addr u -- flag )
  DUP CLEN @ <> IF 2DROP 0 EXIT THEN
  0 DO
    DUP I + C@  CMD CSTART @ + I + C@  <> IF DROP UNLOOP 0 EXIT THEN
  LOOP DROP -1 ;

: SHOW-ROOM ( -- )
  ROOM @ 0 = IF
    ." You are in a small room." CR
    ." Exits: NORTH" CR
  ELSE ROOM @ 1 = IF
    ." You are in a hall." CR
    ." Exits: SOUTH EAST" CR
  ELSE
    ." You found the key." CR
  THEN THEN
;

: TRY-MOVE ( -- )
  ROOM @ 0 = IF
    S" NORTH" CMD= IF 1 ROOM ! EXIT THEN
  THEN
  ROOM @ 1 = IF
    S" SOUTH" CMD= IF 0 ROOM ! EXIT THEN
    S" EAST" CMD= IF 2 ROOM ! EXIT THEN
  THEN
  ." Can't go that way." CR
;

: PLAY ( -- )
  0 ROOM !
  BEGIN
    RUNNING
  WHILE
    SHOW-ROOM
    ." > "
    CMD 80 EXPECT
    RUNNING 0= IF EXIT THEN
    CMD SPAN @ TYPE CR
    PREPARE-CMD
    TRY-MOVE
  REPEAT
;
