unit pforth_types;

{$mode objfpc}{$H+}

interface

uses
  SysUtils;

type
  TCell = SmallInt;
  TUCell = Word;
  TByte = Byte;
  TMemory = array[0..65535] of Byte;
  TCellStack = array of TCell;

  EForthError = class(Exception);

const
  MEM_SIZE = 65536;
  SPAN_ADDR: TUCell = 1000;
  DP_ADDR: TUCell = 1002;
  BASE_ADDR: TUCell = 1004;
  INITIAL_DP: TUCell = 1024;

implementation

end.
