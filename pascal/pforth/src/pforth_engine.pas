unit pforth_engine;

{$mode objfpc}{$H+}

interface

uses
  SysUtils, Classes, pforth_types;

type
  TForth = class
  public
    DS: TCellStack;
    RS: TCellStack;
    Memory: TMemory;
    Running: Boolean;
    Interrupted: Boolean;
    InputStream: TStream;
    OutputStream: TStream;

    constructor Create(AInput: TStream = nil; AOutput: TStream = nil);
    destructor Destroy; override;

    // Stack operations
    procedure PushDS(V: TCell);
    function PopDS: TCell;
    function PeekDS: TCell;
    function DepthDS: Integer;

    procedure PushRS(V: TCell);
    function PopRS: TCell;
    function PeekRS: TCell;
    function DepthRS: Integer;

    // Output helper
    procedure EmitChar(Ch: Char);
    procedure EmitString(const S: String);

    // Primitives for Core I/O (001.2)
    procedure WordKey;
    procedure WordEmit;
    procedure WordCR;
    procedure WordSpace;
    procedure WordSpaces;
    procedure WordType;
    procedure WordBye;
  end;

implementation

constructor TForth.Create(AInput: TStream; AOutput: TStream);
begin
  inherited Create;
  SetLength(DS, 0);
  SetLength(RS, 0);
  FillChar(Memory, SizeOf(Memory), 0);
  Running := True;
  Interrupted := False;
  InputStream := AInput;
  OutputStream := AOutput;

  // Initialize standard address values
  Memory[SPAN_ADDR] := 0;
  Memory[SPAN_ADDR + 1] := 0;
  Memory[DP_ADDR] := Byte(INITIAL_DP and $FF);
  Memory[DP_ADDR + 1] := Byte((INITIAL_DP shr 8) and $FF);
  Memory[BASE_ADDR] := 10;
  Memory[BASE_ADDR + 1] := 0;
end;

destructor TForth.Destroy;
begin
  SetLength(DS, 0);
  SetLength(RS, 0);
  inherited Destroy;
end;

procedure TForth.PushDS(V: TCell);
var
  Len: Integer;
begin
  Len := Length(DS);
  SetLength(DS, Len + 1);
  DS[Len] := V;
end;

function TForth.PopDS: TCell;
var
  Len: Integer;
begin
  Len := Length(DS);
  if Len = 0 then
    raise EForthError.Create('DATA STACK UNDERFLOW');
  Result := DS[Len - 1];
  SetLength(DS, Len - 1);
end;

function TForth.PeekDS: TCell;
var
  Len: Integer;
begin
  Len := Length(DS);
  if Len = 0 then
    raise EForthError.Create('DATA STACK UNDERFLOW');
  Result := DS[Len - 1];
end;

function TForth.DepthDS: Integer;
begin
  Result := Length(DS);
end;

procedure TForth.PushRS(V: TCell);
var
  Len: Integer;
begin
  Len := Length(RS);
  SetLength(RS, Len + 1);
  RS[Len] := V;
end;

function TForth.PopRS: TCell;
var
  Len: Integer;
begin
  Len := Length(RS);
  if Len = 0 then
    raise EForthError.Create('RETURN STACK UNDERFLOW');
  Result := RS[Len - 1];
  SetLength(RS, Len - 1);
end;

function TForth.PeekRS: TCell;
var
  Len: Integer;
begin
  Len := Length(RS);
  if Len = 0 then
    raise EForthError.Create('RETURN STACK UNDERFLOW');
  Result := RS[Len - 1];
end;

function TForth.DepthRS: Integer;
begin
  Result := Length(RS);
end;

procedure TForth.EmitChar(Ch: Char);
var
  B: Byte;
begin
  B := Byte(Ord(Ch));
  if OutputStream <> nil then
    OutputStream.Write(B, 1)
  else
    Write(Ch);
end;

procedure TForth.EmitString(const S: String);
var
  I: Integer;
begin
  for I := 1 to Length(S) do
    EmitChar(S[I]);
end;

procedure TForth.WordKey;
var
  B: Byte;
  Ch: Char;
begin
  if (InputStream <> nil) and (InputStream.Position < InputStream.Size) then
  begin
    if InputStream.Read(B, 1) = 1 then
      PushDS(TCell(B))
    else
      PushDS(0);
  end
  else if InputStream <> nil then
    PushDS(0)
  else
  begin
    if not Eof then
    begin
      Read(Ch);
      PushDS(TCell(Ord(Ch)));
    end
    else
      PushDS(0);
  end;
end;

procedure TForth.WordEmit;
var
  C: TCell;
begin
  C := PopDS;
  EmitChar(Chr(Byte(C and $FF)));
end;

procedure TForth.WordCR;
begin
  EmitChar(#10);
end;

procedure TForth.WordSpace;
begin
  EmitChar(' ');
end;

procedure TForth.WordSpaces;
var
  N: TCell;
  I: Integer;
begin
  N := PopDS;
  if N > 0 then
    for I := 1 to N do
      WordSpace;
end;

procedure TForth.WordType;
var
  LenVal, AddrVal: TCell;
  UAddr: LongWord;
  I: Integer;
begin
  LenVal := PopDS;
  AddrVal := PopDS;
  if LenVal <= 0 then Exit;
  UAddr := LongWord(TUCell(AddrVal));
  for I := 0 to LenVal - 1 do
  begin
    if UAddr + LongWord(I) <= 65535 then
      EmitChar(Chr(Memory[UAddr + LongWord(I)]));
  end;
end;

procedure TForth.WordBye;
begin
  Running := False;
end;

end.
