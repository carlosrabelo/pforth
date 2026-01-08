unit pforth_engine;

{$mode objfpc}{$H+}

interface

uses
  SysUtils, Classes, pforth_types;

type
  TStringSet = array of String;

  TForth = class
  public
    DS: TCellStack;
    RS: TCellStack;
    Memory: TMemory;
    Running: Boolean;
    Interrupted: Boolean;
    State: Boolean;
    CompilingXT: Integer;
    CompileList: TCellStack;
    InputStream: TStream;
    OutputStream: TStream;
    TIB: String;
    IN_PTR: Integer;
    LoadedModules: TStringSet;

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

    // Memory helpers
    procedure StoreCell(Addr: Integer; Val: TCell);
    function FetchCell(Addr: Integer): TCell;
    procedure CStore(Addr: Integer; Val: Byte);
    function CFetch(Addr: Integer): Byte;

    // Output helper
    procedure EmitChar(Ch: Char);
    procedure EmitString(const S: String);

    // Line reader & TIB (001.3)
    procedure SetLine(const ALine: String);
    function ReadLine(out Line: String): Boolean;

    // Primitives for Core I/O (001.2, 001.3)
    procedure WordKey;
    procedure WordEmit;
    procedure WordCR;
    procedure WordSpace;
    procedure WordSpaces;
    procedure WordType;
    procedure WordBye;
    procedure WordSpan;
    procedure WordExpect;

    // Memory words (002.3, 002.4)
    procedure WordFetch;
    procedure WordStore;
    procedure WordCFetch;
    procedure WordCStore;
    procedure WordPlusStore;
    procedure WordFill;
    procedure WordErase;
    procedure WordCMove;
    procedure WordCMoveGreater;

    // Stack words (003)
    procedure WordDup;
    procedure WordDrop;
    procedure WordSwap;
    procedure WordOver;

    // REPL / cold start (001.4)
    procedure InitPrimitives;
    procedure RequireModule(const ModuleName: String);
    procedure Abort;
    procedure CheckInterrupt;
    procedure InterpretLine(const ALine: String); virtual;
    procedure ReplStep;
    procedure Quit;
  end;

implementation

constructor TForth.Create(AInput: TStream; AOutput: TStream);
begin
  inherited Create;
  SetLength(DS, 0);
  SetLength(RS, 0);
  SetLength(CompileList, 0);
  SetLength(LoadedModules, 0);
  FillChar(Memory, SizeOf(Memory), 0);
  Running := True;
  Interrupted := False;
  State := False;
  CompilingXT := -1;
  InputStream := AInput;
  OutputStream := AOutput;

  // Initialize standard address values
  Memory[SPAN_ADDR] := 0;
  Memory[SPAN_ADDR + 1] := 0;
  Memory[DP_ADDR] := Byte(INITIAL_DP and $FF);
  Memory[DP_ADDR + 1] := Byte((INITIAL_DP shr 8) and $FF);
  Memory[BASE_ADDR] := 10;
  Memory[BASE_ADDR + 1] := 0;

  InitPrimitives;
  RequireModule('CORE');
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

procedure TForth.StoreCell(Addr: Integer; Val: TCell);
begin
  if (Addr < 0) or (Addr > 65534) then
    raise EForthError.Create('OUT OF MEMORY RANGE');
  Memory[Addr] := Byte(TUCell(Val) and $FF);
  Memory[Addr + 1] := Byte((TUCell(Val) shr 8) and $FF);
end;

function TForth.FetchCell(Addr: Integer): TCell;
var
  LowB, HighB: Byte;
begin
  if (Addr < 0) or (Addr > 65534) then
    raise EForthError.Create('OUT OF MEMORY RANGE');
  LowB := Memory[Addr];
  HighB := Memory[Addr + 1];
  Result := TCell(Word(LowB) or (Word(HighB) shl 8));
end;

procedure TForth.CStore(Addr: Integer; Val: Byte);
begin
  if (Addr < 0) or (Addr > 65535) then
    raise EForthError.Create('OUT OF MEMORY RANGE');
  Memory[Addr] := Val;
end;

function TForth.CFetch(Addr: Integer): Byte;
begin
  if (Addr < 0) or (Addr > 65535) then
    raise EForthError.Create('OUT OF MEMORY RANGE');
  Result := Memory[Addr];
end;

procedure TForth.SetLine(const ALine: String);
begin
  TIB := ALine;
  IN_PTR := 1;
end;

function TForth.ReadLine(out Line: String): Boolean;
var
  B: Byte;
  Ch: Char;
  Str: String;
begin
  Str := '';
  if InputStream <> nil then
  begin
    if InputStream.Position >= InputStream.Size then
    begin
      Running := False;
      Line := '';
      Result := False;
      Exit;
    end;
    while InputStream.Position < InputStream.Size do
    begin
      InputStream.Read(B, 1);
      Ch := Chr(B);
      if Ch = #13 then
      begin
        if InputStream.Position < InputStream.Size then
        begin
          InputStream.Read(B, 1);
          if Chr(B) <> #10 then
            InputStream.Seek(-1, soFromCurrent);
        end;
        Break;
      end;
      if Ch = #10 then
        Break;
      Str := Str + Ch;
    end;
    Line := Str;
    Result := True;
  end
  else
  begin
    if Eof then
    begin
      Running := False;
      Line := '';
      Result := False;
    end
    else
    begin
      ReadLn(Line);
      Result := True;
    end;
  end;
end;

procedure TForth.WordSpan;
begin
  PushDS(TCell(SPAN_ADDR));
end;

procedure TForth.WordExpect;
var
  MaxLen, AddrVal: TCell;
  UAddr: LongWord;
  Count: Integer;
  Ch: Char;
  B: Byte;
begin
  MaxLen := PopDS;
  AddrVal := PopDS;
  if MaxLen <= 0 then
  begin
    StoreCell(SPAN_ADDR, 0);
    Exit;
  end;
  UAddr := LongWord(TUCell(AddrVal));
  Count := 0;
  while Count < MaxLen do
  begin
    if InputStream <> nil then
    begin
      if InputStream.Position >= InputStream.Size then
        Break;
      InputStream.Read(B, 1);
      Ch := Chr(B);
    end
    else
    begin
      if Eof then Break;
      Read(Ch);
    end;

    if Ch = #13 then
    begin
      if InputStream <> nil then
      begin
        if InputStream.Position < InputStream.Size then
        begin
          InputStream.Read(B, 1);
          if Chr(B) <> #10 then
            InputStream.Seek(-1, soFromCurrent);
        end;
      end;
      Break;
    end;
    if Ch = #10 then
      Break;

    if UAddr + LongWord(Count) <= 65535 then
      Memory[UAddr + LongWord(Count)] := Byte(Ord(Ch));
    Inc(Count);
  end;
  StoreCell(SPAN_ADDR, TCell(Count));
end;

function ZeroExtendAddr(V: TCell): Integer;
begin
  Result := Integer(TUCell(V));
end;

procedure TForth.WordFetch;
begin
  PushDS(FetchCell(ZeroExtendAddr(PopDS)));
end;

procedure TForth.WordStore;
var
  Addr: Integer;
  Val: TCell;
begin
  Addr := ZeroExtendAddr(PopDS);
  Val := PopDS;
  StoreCell(Addr, Val);
end;

procedure TForth.WordCFetch;
begin
  PushDS(TCell(CFetch(ZeroExtendAddr(PopDS))));
end;

procedure TForth.WordCStore;
var
  Addr: Integer;
  Val: TCell;
begin
  Addr := ZeroExtendAddr(PopDS);
  Val := PopDS;
  CStore(Addr, Byte(TUCell(Val) and $FF));
end;

procedure TForth.WordPlusStore;
var
  Addr: Integer;
  N: TCell;
begin
  Addr := ZeroExtendAddr(PopDS);
  N := PopDS;
  StoreCell(Addr, FetchCell(Addr) + N);
end;

procedure TForth.WordFill;
var
  Val: Byte;
  Len, Addr, I: Integer;
begin
  Val := Byte(TUCell(PopDS) and $FF);
  Len := Integer(PopDS);
  Addr := ZeroExtendAddr(PopDS);
  if Len <= 0 then
    Exit;
  for I := 0 to Len - 1 do
    CStore(Addr + I, Val);
end;

procedure TForth.WordErase;
var
  Len, Addr, I: Integer;
begin
  Len := Integer(PopDS);
  Addr := ZeroExtendAddr(PopDS);
  if Len <= 0 then
    Exit;
  for I := 0 to Len - 1 do
    CStore(Addr + I, 0);
end;

procedure TForth.WordCMove;
var
  Len, Src, Dst, I: Integer;
begin
  Len := Integer(PopDS);
  Src := ZeroExtendAddr(PopDS);
  Dst := ZeroExtendAddr(PopDS);
  if Len <= 0 then
    Exit;
  for I := 0 to Len - 1 do
    CStore(Dst + I, CFetch(Src + I));
end;

procedure TForth.WordCMoveGreater;
var
  Len, Src, Dst, I: Integer;
begin
  Len := Integer(PopDS);
  Src := ZeroExtendAddr(PopDS);
  Dst := ZeroExtendAddr(PopDS);
  if Len <= 0 then
    Exit;
  for I := Len - 1 downto 0 do
    CStore(Dst + I, CFetch(Src + I));
end;

procedure TForth.WordDup;
begin
  PushDS(PeekDS);
end;

procedure TForth.WordDrop;
begin
  PopDS;
end;

procedure TForth.WordSwap;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(B);
  PushDS(A);
end;

procedure TForth.WordOver;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(A);
  PushDS(B);
  PushDS(A);
end;

procedure TForth.InitPrimitives;
begin
  { Kernel registration grows in later specs. }
end;

procedure TForth.RequireModule(const ModuleName: String);
var
  Upper: String;
  I: Integer;
begin
  Upper := UpperCase(ModuleName);
  for I := 0 to High(LoadedModules) do
    if LoadedModules[I] = Upper then
      Exit;
  SetLength(LoadedModules, Length(LoadedModules) + 1);
  LoadedModules[High(LoadedModules)] := Upper;
  if Upper = 'CORE' then
  begin
    { CORE colon library is installed in later specs. }
  end;
end;

procedure TForth.Abort;
begin
  SetLength(DS, 0);
  SetLength(RS, 0);
  SetLength(CompileList, 0);
  CompilingXT := -1;
  State := False;
  Interrupted := False;
  if Length(TIB) = 0 then
    IN_PTR := 1
  else
    IN_PTR := Length(TIB) + 1;
end;

procedure TForth.CheckInterrupt;
begin
  if Interrupted then
  begin
    Interrupted := False;
    Abort;
    raise EForthError.Create('INTERRUPTED');
  end;
end;

procedure TForth.InterpretLine(const ALine: String);
begin
  SetLine(ALine);
  { Outer interpreter is filled in later specs. }
end;

procedure TForth.ReplStep;
var
  Line: String;
begin
  try
    if not State then
      EmitString(' ok' + #10);
    if not ReadLine(Line) then
      Exit;
    CheckInterrupt;
    InterpretLine(Line);
  except
    on E: EForthError do
    begin
      EmitString(E.Message + #10);
      Abort;
    end;
  end;
end;

procedure TForth.Quit;
begin
  Running := True;
  Abort;
  while Running do
    ReplStep;
end;

end.
