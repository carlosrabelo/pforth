unit pforth_engine;

{$mode objfpc}{$H+}

interface

uses
  SysUtils, Classes, pforth_types;

type
  TStringSet = array of String;
  TWordKind = (wkPrimitive, wkColon, wkConstant, wkVariable, wkCreate);
  TForthWordCode = procedure of object;

  TForthWord = record
    Name: String;
    Immediate: Boolean;
    Kind: TWordKind;
    Code: TForthWordCode;
    Body: TCellStack;
    PFA: Integer;
  end;

  TForthWordArray = array of TForthWord;

  TForth = class
  public
    DS: TCellStack;
    RS: TCellStack;
    Memory: TMemory;
    Words: TForthWordArray;
    Latest: Integer;
    CurrentBody: TCellStack;
    IP: Integer;
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
    procedure WordQDup;
    procedure WordRot;
    procedure WordNRot;
    procedure WordNip;
    procedure WordTuck;
    procedure WordTwoDup;
    procedure WordTwoDrop;
    procedure WordTwoSwap;
    procedure WordTwoOver;
    procedure WordToR;
    procedure WordRFrom;
    procedure WordRFetch;

    // Arithmetic and logic (004)
    procedure WordPlus;
    procedure WordMinus;
    procedure WordMultiply;
    procedure WordSlashMod;
    procedure WordSlash;
    procedure WordMod;
    procedure WordNegate;
    procedure WordOnePlus;
    procedure WordOneMinus;
    procedure WordTwoPlus;
    procedure WordTwoMinus;
    procedure WordTwoStar;
    procedure WordTwoSlash;
    procedure WordAbs;
    procedure WordMin;
    procedure WordMax;
    procedure WordAnd;
    procedure WordOr;
    procedure WordXor;
    procedure WordInvert;
    procedure WordEquals;
    procedure WordNotEquals;
    procedure WordLess;
    procedure WordGreater;
    procedure WordLessEqual;
    procedure WordGreaterEqual;
    procedure WordZeroEquals;
    procedure WordZeroNotEquals;
    procedure WordZeroLess;
    procedure WordZeroGreater;
    procedure WordULess;
    procedure WordUGreater;
    procedure WordULessEqual;
    procedure WordUGreaterEqual;

    // Dictionary (005)
    function DefineWord(const AName: String; AKind: TWordKind; ACode: TForthWordCode;
      const ABody: TCellStack; AImmediate: Boolean): Integer;
    function DefinePrimitive(const AName: String; ACode: TForthWordCode): Integer;
    function DefineImmediate(const AName: String; ACode: TForthWordCode): Integer;
    function Lookup(const AName: String; out W: TForthWord): Boolean;
    function FindXT(const AName: String; out XT: Integer): Boolean;
    function ParseWord(out Token: String): Boolean;
    procedure WordFind;
    procedure WordTick;
    procedure WordToBody;
    function Here: Integer;
    procedure Allot(N: Integer);
    procedure Comma(Val: TCell);
    procedure CComma(Val: Byte);
    procedure WordDP;
    procedure WordHere;
    procedure WordPAD;
    procedure WordAllot;
    procedure WordComma;
    procedure WordCComma;
    procedure ExecuteWord(XT: Integer);
    procedure WordExecute;
    procedure WordExit;
    procedure WordCreate;
    procedure WordConstant;
    procedure WordVariable;

    // Outer interpreter / compiler (006)
    function GetBase: Integer;
    function ParseNumber(const Token: String; out Value: TCell): Boolean;
    procedure WordBase;
    procedure WordWord;
    procedure WordNumber;
    procedure WordLIT;
    procedure InterpretLoop;
    procedure WordInterpret;
    procedure WordState;
    procedure WordLeftBracket;
    procedure WordRightBracket;
    procedure WordLiteral;
    procedure WordCompile;
    procedure WordBracketCompile;
    procedure WordColon;
    procedure WordSemicolon;
    procedure WordImmediate;
    procedure WordRecurse;
    function EnsureLitXT: Integer;
    function EnsureExitXT: Integer;
    function EnsureNamedPrimitive(const AName: String; ACode: TForthWordCode): Integer;
    procedure CompileAppend(V: TCell);

    // Control flow (007)
    procedure WordBranch;
    procedure WordZeroBranch;
    procedure WordIF;
    procedure WordELSE;
    procedure WordTHEN;
    procedure WordBEGIN;
    procedure WordUNTIL;
    procedure WordAGAIN;
    procedure WordWHILE;
    procedure WordREPEAT;
    procedure WordDoRun;
    procedure WordLoopRun;
    procedure WordPlusLoopRun;
    procedure WordLeaveRun;
    procedure WordDo;
    procedure WordLoop;
    procedure WordPlusLoop;
    procedure WordI;
    procedure WordJ;
    procedure WordLeave;
    procedure WordUnloop;

    // Numeric output (008.1)
    function FormatCell(Val: TCell; ABase: Integer; IsSigned: Boolean): String;
    procedure WordDot;
    procedure WordUDot;
    procedure WordDotS;
    procedure WordPage;

    // Strings and comments (008.2)
    function ParseStringLiteral: String;
    procedure WordDotQuote;
    procedure WordSQuote;
    procedure WordParen;
    procedure WordBackslash;

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
  SetLength(Words, 0);
  SetLength(CurrentBody, 0);
  FillChar(Memory, SizeOf(Memory), 0);
  Running := True;
  Interrupted := False;
  State := False;
  CompilingXT := -1;
  Latest := -1;
  IP := 0;
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

function WrapCell(V: Integer): TCell;
begin
  Result := TCell(Word(V and $FFFF));
end;

function FlagCell(Cond: Boolean): TCell;
begin
  if Cond then
    Result := -1
  else
    Result := 0;
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

procedure TForth.WordQDup;
var
  V: TCell;
begin
  V := PeekDS;
  if V <> 0 then
    PushDS(V);
end;

procedure TForth.WordRot;
var
  A, B, C: TCell;
begin
  C := PopDS;
  B := PopDS;
  A := PopDS;
  PushDS(B);
  PushDS(C);
  PushDS(A);
end;

procedure TForth.WordNRot;
var
  A, B, C: TCell;
begin
  C := PopDS;
  B := PopDS;
  A := PopDS;
  PushDS(C);
  PushDS(A);
  PushDS(B);
end;

procedure TForth.WordNip;
var
  B: TCell;
begin
  B := PopDS;
  PopDS;
  PushDS(B);
end;

procedure TForth.WordTuck;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(B);
  PushDS(A);
  PushDS(B);
end;

procedure TForth.WordTwoDup;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(A);
  PushDS(B);
  PushDS(A);
  PushDS(B);
end;

procedure TForth.WordTwoDrop;
begin
  PopDS;
  PopDS;
end;

procedure TForth.WordTwoSwap;
var
  A, B, C, D: TCell;
begin
  D := PopDS;
  C := PopDS;
  B := PopDS;
  A := PopDS;
  PushDS(C);
  PushDS(D);
  PushDS(A);
  PushDS(B);
end;

procedure TForth.WordTwoOver;
var
  A, B, C, D: TCell;
begin
  D := PopDS;
  C := PopDS;
  B := PopDS;
  A := PopDS;
  PushDS(A);
  PushDS(B);
  PushDS(C);
  PushDS(D);
  PushDS(A);
  PushDS(B);
end;

procedure TForth.WordToR;
begin
  PushRS(PopDS);
end;

procedure TForth.WordRFrom;
begin
  PushDS(PopRS);
end;

procedure TForth.WordRFetch;
begin
  PushDS(PeekRS);
end;

procedure TForth.WordPlus;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(WrapCell(Integer(A) + Integer(B)));
end;

procedure TForth.WordMinus;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(WrapCell(Integer(A) - Integer(B)));
end;

procedure TForth.WordMultiply;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(WrapCell(Integer(A) * Integer(B)));
end;

procedure TForth.WordSlashMod;
var
  A, B, Quot, Rem: TCell;
begin
  B := PopDS;
  A := PopDS;
  if B = 0 then
    raise EForthError.Create('DIVISION BY ZERO');
  Quot := WrapCell(Integer(A) div Integer(B));
  Rem := WrapCell(Integer(A) - Integer(Quot) * Integer(B));
  PushDS(Rem);
  PushDS(Quot);
end;

procedure TForth.WordSlash;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  if B = 0 then
    raise EForthError.Create('DIVISION BY ZERO');
  PushDS(WrapCell(Integer(A) div Integer(B)));
end;

procedure TForth.WordMod;
var
  A, B, Quot: TCell;
begin
  B := PopDS;
  A := PopDS;
  if B = 0 then
    raise EForthError.Create('DIVISION BY ZERO');
  Quot := WrapCell(Integer(A) div Integer(B));
  PushDS(WrapCell(Integer(A) - Integer(Quot) * Integer(B)));
end;

procedure TForth.WordNegate;
begin
  PushDS(WrapCell(-Integer(PopDS)));
end;

procedure TForth.WordOnePlus;
begin
  PushDS(WrapCell(Integer(PopDS) + 1));
end;

procedure TForth.WordOneMinus;
begin
  PushDS(WrapCell(Integer(PopDS) - 1));
end;

procedure TForth.WordTwoPlus;
begin
  PushDS(WrapCell(Integer(PopDS) + 2));
end;

procedure TForth.WordTwoMinus;
begin
  PushDS(WrapCell(Integer(PopDS) - 2));
end;

procedure TForth.WordTwoStar;
begin
  PushDS(WrapCell(Integer(PopDS) shl 1));
end;

procedure TForth.WordTwoSlash;
var
  W: Word;
begin
  W := Word(PopDS);
  if (W and $8000) <> 0 then
    PushDS(TCell((W shr 1) or $8000))
  else
    PushDS(TCell(W shr 1));
end;

procedure TForth.WordAbs;
var
  N: TCell;
begin
  N := PopDS;
  if N < 0 then
    PushDS(WrapCell(-Integer(N)))
  else
    PushDS(N);
end;

procedure TForth.WordMin;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  if A < B then
    PushDS(A)
  else
    PushDS(B);
end;

procedure TForth.WordMax;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  if A > B then
    PushDS(A)
  else
    PushDS(B);
end;

procedure TForth.WordAnd;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(TCell(Word(A) and Word(B)));
end;

procedure TForth.WordOr;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(TCell(Word(A) or Word(B)));
end;

procedure TForth.WordXor;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(TCell(Word(A) xor Word(B)));
end;

procedure TForth.WordInvert;
begin
  PushDS(TCell(not Word(PopDS)));
end;

procedure TForth.WordEquals;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(FlagCell(A = B));
end;

procedure TForth.WordNotEquals;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(FlagCell(A <> B));
end;

procedure TForth.WordLess;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(FlagCell(A < B));
end;

procedure TForth.WordGreater;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(FlagCell(A > B));
end;

procedure TForth.WordLessEqual;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(FlagCell(A <= B));
end;

procedure TForth.WordGreaterEqual;
var
  A, B: TCell;
begin
  B := PopDS;
  A := PopDS;
  PushDS(FlagCell(A >= B));
end;

procedure TForth.WordZeroEquals;
begin
  PushDS(FlagCell(PopDS = 0));
end;

procedure TForth.WordZeroNotEquals;
begin
  PushDS(FlagCell(PopDS <> 0));
end;

procedure TForth.WordZeroLess;
begin
  PushDS(FlagCell(PopDS < 0));
end;

procedure TForth.WordZeroGreater;
begin
  PushDS(FlagCell(PopDS > 0));
end;

procedure TForth.WordULess;
var
  A, B: Word;
begin
  B := Word(PopDS);
  A := Word(PopDS);
  PushDS(FlagCell(A < B));
end;

procedure TForth.WordUGreater;
var
  A, B: Word;
begin
  B := Word(PopDS);
  A := Word(PopDS);
  PushDS(FlagCell(A > B));
end;

procedure TForth.WordULessEqual;
var
  A, B: Word;
begin
  B := Word(PopDS);
  A := Word(PopDS);
  PushDS(FlagCell(A <= B));
end;

procedure TForth.WordUGreaterEqual;
var
  A, B: Word;
begin
  B := Word(PopDS);
  A := Word(PopDS);
  PushDS(FlagCell(A >= B));
end;

function TForth.DefineWord(const AName: String; AKind: TWordKind; ACode: TForthWordCode;
  const ABody: TCellStack; AImmediate: Boolean): Integer;
var
  Idx: Integer;
  W: TForthWord;
begin
  Idx := Length(Words);
  W.Name := UpperCase(AName);
  W.Immediate := AImmediate;
  W.Kind := AKind;
  W.Code := ACode;
  W.Body := Copy(ABody);
  W.PFA := Here;
  SetLength(Words, Idx + 1);
  Words[Idx] := W;
  Latest := Idx;
  Result := Idx;
end;

function TForth.DefinePrimitive(const AName: String; ACode: TForthWordCode): Integer;
var
  Empty: TCellStack;
begin
  SetLength(Empty, 0);
  Result := DefineWord(AName, wkPrimitive, ACode, Empty, False);
end;

function TForth.DefineImmediate(const AName: String; ACode: TForthWordCode): Integer;
var
  Empty: TCellStack;
begin
  SetLength(Empty, 0);
  Result := DefineWord(AName, wkPrimitive, ACode, Empty, True);
end;

function TForth.Lookup(const AName: String; out W: TForthWord): Boolean;
var
  XT: Integer;
begin
  Result := FindXT(AName, XT);
  if Result then
    W := Words[XT];
end;

function TForth.FindXT(const AName: String; out XT: Integer): Boolean;
var
  Upper: String;
  I: Integer;
begin
  Upper := UpperCase(AName);
  for I := High(Words) downto 0 do
    if Words[I].Name = Upper then
    begin
      XT := I;
      Result := True;
      Exit;
    end;
  XT := -1;
  Result := False;
end;

function TForth.ParseWord(out Token: String): Boolean;
var
  Start: Integer;
begin
  while (IN_PTR <= Length(TIB)) and (TIB[IN_PTR] in [' ', #9, #13, #10]) do
    Inc(IN_PTR);
  if IN_PTR > Length(TIB) then
  begin
    Token := '';
    Result := False;
    Exit;
  end;
  Start := IN_PTR;
  while (IN_PTR <= Length(TIB)) and not (TIB[IN_PTR] in [' ', #9, #13, #10]) do
    Inc(IN_PTR);
  Token := Copy(TIB, Start, IN_PTR - Start);
  Result := True;
end;

procedure TForth.WordFind;
var
  Addr, Len, I: Integer;
  Name: String;
  W: TForthWord;
  XT: Integer;
begin
  Addr := ZeroExtendAddr(PopDS);
  Len := Integer(CFetch(Addr));
  Name := '';
  for I := 0 to Len - 1 do
    Name := Name + Chr(CFetch(Addr + 1 + I));
  Name := UpperCase(Name);
  if not Lookup(Name, W) then
  begin
    PushDS(TCell(Addr));
    PushDS(0);
    Exit;
  end;
  FindXT(Name, XT);
  PushDS(TCell(XT));
  if W.Immediate then
    PushDS(1)
  else
    PushDS(-1);
end;

procedure TForth.WordTick;
var
  Token: String;
  XT: Integer;
begin
  if not ParseWord(Token) then
    raise EForthError.Create('WORD NAME EXPECTED');
  if not FindXT(Token, XT) then
    raise EForthError.Create('WORD NOT FOUND: ' + Token);
  PushDS(TCell(XT));
end;

procedure TForth.WordToBody;
var
  XT: Integer;
begin
  XT := Integer(PopDS);
  if (XT >= 0) and (XT < Length(Words)) and (Words[XT].PFA <> 0) then
    PushDS(TCell(Words[XT].PFA))
  else
    PushDS(TCell(XT + 2));
end;

function TForth.Here: Integer;
begin
  Result := Integer(FetchCell(DP_ADDR));
end;

procedure TForth.Allot(N: Integer);
begin
  StoreCell(DP_ADDR, TCell(Here + N));
end;

procedure TForth.Comma(Val: TCell);
begin
  StoreCell(Here, Val);
  Allot(2);
end;

procedure TForth.CComma(Val: Byte);
begin
  CStore(Here, Val);
  Allot(1);
end;

procedure TForth.WordDP;
begin
  PushDS(TCell(DP_ADDR));
end;

procedure TForth.WordHere;
begin
  PushDS(TCell(Here));
end;

procedure TForth.WordPAD;
begin
  PushDS(TCell(Here + 68));
end;

procedure TForth.WordAllot;
begin
  Allot(Integer(PopDS));
end;

procedure TForth.WordComma;
begin
  Comma(PopDS);
end;

procedure TForth.WordCComma;
begin
  CComma(Byte(TUCell(PopDS) and $FF));
end;

procedure TForth.ExecuteWord(XT: Integer);
var
  W: TForthWord;
  SavedBody: TCellStack;
  SavedIP: Integer;
  Nxt: Integer;
begin
  if (XT < 0) or (XT >= Length(Words)) then
    raise EForthError.Create('INVALID EXECUTION TOKEN: ' + IntToStr(XT));
  W := Words[XT];
  case W.Kind of
    wkPrimitive:
      if Assigned(W.Code) then
        W.Code();
    wkConstant:
      if Length(W.Body) > 0 then
        PushDS(W.Body[0]);
    wkVariable, wkCreate:
      PushDS(TCell(W.PFA));
    wkColon:
      begin
        SavedBody := CurrentBody;
        SavedIP := IP;
        try
          CurrentBody := W.Body;
          IP := 0;
          while IP < Length(CurrentBody) do
          begin
            CheckInterrupt;
            Nxt := Integer(CurrentBody[IP]);
            Inc(IP);
            ExecuteWord(Nxt);
          end;
        finally
          CurrentBody := SavedBody;
          IP := SavedIP;
        end;
      end;
  end;
end;

procedure TForth.WordExecute;
begin
  ExecuteWord(Integer(PopDS));
end;

procedure TForth.WordExit;
begin
  IP := Length(CurrentBody);
end;

procedure TForth.WordCreate;
var
  Token: String;
  Empty: TCellStack;
begin
  if not ParseWord(Token) then
    raise EForthError.Create('WORD NAME EXPECTED');
  SetLength(Empty, 0);
  DefineWord(Token, wkCreate, nil, Empty, False);
end;

procedure TForth.WordConstant;
var
  Token: String;
  N: TCell;
  Body: TCellStack;
begin
  N := PopDS;
  if not ParseWord(Token) then
    raise EForthError.Create('WORD NAME EXPECTED');
  SetLength(Body, 1);
  Body[0] := N;
  DefineWord(Token, wkConstant, nil, Body, False);
end;

procedure TForth.WordVariable;
var
  Token: String;
  Empty: TCellStack;
  PFA: Integer;
begin
  if not ParseWord(Token) then
    raise EForthError.Create('WORD NAME EXPECTED');
  PFA := Here;
  StoreCell(PFA, 0);
  Allot(2);
  SetLength(Empty, 0);
  DefineWord(Token, wkVariable, nil, Empty, False);
  { DefineWord snapshots PFA via Here after allot — fix PFA to pre-allot address. }
  Words[Latest].PFA := PFA;
end;

function TForth.GetBase: Integer;
begin
  Result := Integer(FetchCell(BASE_ADDR));
  if Result < 1 then
    Result := 10;
end;

function TForth.ParseNumber(const Token: String; out Value: TCell): Boolean;
var
  Neg: Boolean;
  Start, ParsedBase, I, Digit: Integer;
  C: Char;
  Acc: LongWord;
begin
  Result := False;
  Value := 0;
  if Length(Token) = 0 then
    Exit;
  Neg := False;
  Start := 1;
  if Token[1] = '-' then
  begin
    Neg := True;
    Start := 2;
  end
  else if Token[1] = '+' then
    Start := 2;
  if Start > Length(Token) then
    Exit;
  ParsedBase := GetBase;
  if Token[Start] = '$' then
  begin
    ParsedBase := 16;
    Inc(Start);
  end
  else if Token[Start] = '%' then
  begin
    ParsedBase := 2;
    Inc(Start);
  end
  else if Token[Start] = '#' then
  begin
    Inc(Start);
    if (Start <= Length(Token)) and (Token[Start] = '#') then
      Inc(Start);
  end;
  if Start > Length(Token) then
    Exit;
  Acc := 0;
  for I := Start to Length(Token) do
  begin
    C := Token[I];
    if (C >= '0') and (C <= '9') then
      Digit := Ord(C) - Ord('0')
    else if (C >= 'A') and (C <= 'F') then
      Digit := Ord(C) - Ord('A') + 10
    else if (C >= 'a') and (C <= 'f') then
      Digit := Ord(C) - Ord('a') + 10
    else
      Exit;
    if Digit >= ParsedBase then
      Exit;
    Acc := Acc * LongWord(ParsedBase) + LongWord(Digit);
  end;
  Value := TCell(Word(Acc and $FFFF));
  if Neg then
    Value := WrapCell(-Integer(Value));
  Result := True;
end;

procedure TForth.WordBase;
begin
  PushDS(TCell(BASE_ADDR));
end;

procedure TForth.WordWord;
var
  Delim: Byte;
  Dest, StartAddr, Count: Integer;
begin
  Delim := Byte(TUCell(PopDS) and $FF);
  while (IN_PTR <= Length(TIB)) and (Byte(Ord(TIB[IN_PTR])) = Delim) do
    Inc(IN_PTR);
  Dest := Here;
  if IN_PTR > Length(TIB) then
  begin
    CStore(Dest, 0);
    PushDS(TCell(Dest));
    Exit;
  end;
  StartAddr := Dest + 1;
  Count := 0;
  while (IN_PTR <= Length(TIB)) and (Byte(Ord(TIB[IN_PTR])) <> Delim) and
    (TIB[IN_PTR] <> #13) and (TIB[IN_PTR] <> #10) do
  begin
    CStore(StartAddr + Count, Byte(Ord(TIB[IN_PTR])));
    Inc(Count);
    Inc(IN_PTR);
  end;
  CStore(Dest, Byte(Count));
  CStore(StartAddr + Count, Ord(' '));
  PushDS(TCell(Dest));
end;

procedure TForth.WordNumber;
var
  Addr, Len, I: Integer;
  Token: String;
  Num: TCell;
begin
  Addr := ZeroExtendAddr(PopDS);
  Len := Integer(CFetch(Addr));
  Token := '';
  for I := 0 to Len - 1 do
    Token := Token + Chr(CFetch(Addr + 1 + I));
  if ParseNumber(Token, Num) then
  begin
    PushDS(Num);
    PushDS(-1);
  end
  else
  begin
    PushDS(0);
    PushDS(0);
  end;
end;

function TForth.EnsureLitXT: Integer;
begin
  if not FindXT('LIT', Result) then
    Result := DefinePrimitive('LIT', @WordLIT);
end;

function TForth.EnsureExitXT: Integer;
begin
  if not FindXT('EXIT', Result) then
    Result := DefinePrimitive('EXIT', @WordExit);
end;

procedure TForth.WordLIT;
begin
  if IP < Length(CurrentBody) then
  begin
    PushDS(CurrentBody[IP]);
    Inc(IP);
  end;
end;

procedure TForth.InterpretLoop;
var
  Token: String;
  Upper: String;
  W: TForthWord;
  XT: Integer;
  Num: TCell;
  ErrToken: String;
begin
  while True do
  begin
    CheckInterrupt;
    if not ParseWord(Token) then
      Break;
    Upper := UpperCase(Token);
    if Lookup(Upper, W) then
    begin
      FindXT(Upper, XT);
      if State and (not W.Immediate) then
      begin
        SetLength(CompileList, Length(CompileList) + 1);
        CompileList[High(CompileList)] := TCell(XT);
      end
      else
        ExecuteWord(XT);
    end
    else if ParseNumber(Token, Num) then
    begin
      if State then
      begin
        XT := EnsureLitXT;
        SetLength(CompileList, Length(CompileList) + 2);
        CompileList[High(CompileList) - 1] := TCell(XT);
        CompileList[High(CompileList)] := Num;
      end
      else
        PushDS(Num);
    end
    else
    begin
      ErrToken := Token;
      Abort;
      raise EForthError.Create('UNKNOWN WORD: ' + ErrToken);
    end;
  end;
end;

procedure TForth.WordInterpret;
begin
  InterpretLoop;
end;

procedure TForth.WordState;
begin
  PushDS(FlagCell(State));
end;

procedure TForth.WordLeftBracket;
begin
  State := False;
end;

procedure TForth.WordRightBracket;
begin
  State := True;
end;

procedure TForth.WordLiteral;
var
  Val: TCell;
  XT: Integer;
begin
  Val := PopDS;
  XT := EnsureLitXT;
  SetLength(CompileList, Length(CompileList) + 2);
  CompileList[High(CompileList) - 1] := TCell(XT);
  CompileList[High(CompileList)] := Val;
end;

procedure TForth.WordCompile;
var
  Token: String;
  XT: Integer;
begin
  if not ParseWord(Token) then
    raise EForthError.Create('WORD NAME EXPECTED');
  if not FindXT(Token, XT) then
    raise EForthError.Create('WORD NOT FOUND: ' + Token);
  SetLength(CompileList, Length(CompileList) + 1);
  CompileList[High(CompileList)] := TCell(XT);
end;

procedure TForth.WordBracketCompile;
begin
  WordCompile;
end;

procedure TForth.WordColon;
var
  Token: String;
  Empty: TCellStack;
begin
  if State then
    raise EForthError.Create('ALREADY COMPILING');
  if not ParseWord(Token) then
    raise EForthError.Create('WORD NAME EXPECTED');
  SetLength(CompileList, 0);
  SetLength(Empty, 0);
  DefineWord(Token, wkColon, nil, Empty, False);
  CompilingXT := Latest;
  State := True;
end;

procedure TForth.WordSemicolon;
var
  ExitXT: Integer;
begin
  if not State then
    raise EForthError.Create('NOT IN COMPILATION STATE');
  ExitXT := EnsureExitXT;
  SetLength(CompileList, Length(CompileList) + 1);
  CompileList[High(CompileList)] := TCell(ExitXT);
  State := False;
  if (CompilingXT >= 0) and (CompilingXT < Length(Words)) then
    Words[CompilingXT].Body := Copy(CompileList);
  SetLength(CompileList, 0);
  CompilingXT := -1;
end;

procedure TForth.WordImmediate;
begin
  if Latest >= 0 then
    Words[Latest].Immediate := True;
end;

procedure TForth.WordRecurse;
begin
  if not State then
    raise EForthError.Create('RECURSE ONLY IN COMPILATION');
  if CompilingXT < 0 then
    raise EForthError.Create('NO COMPILATION IN PROGRESS');
  CompileAppend(TCell(CompilingXT));
end;

function TForth.EnsureNamedPrimitive(const AName: String; ACode: TForthWordCode): Integer;
begin
  if not FindXT(AName, Result) then
    Result := DefinePrimitive(AName, ACode);
end;

procedure TForth.CompileAppend(V: TCell);
begin
  SetLength(CompileList, Length(CompileList) + 1);
  CompileList[High(CompileList)] := V;
end;

procedure TForth.WordBranch;
var
  Offset: Integer;
begin
  if IP < Length(CurrentBody) then
  begin
    Offset := Integer(CurrentBody[IP]);
    Inc(IP, Offset);
  end;
end;

procedure TForth.WordZeroBranch;
var
  Flag: TCell;
  Offset: Integer;
begin
  Flag := PopDS;
  if IP < Length(CurrentBody) then
  begin
    Offset := Integer(CurrentBody[IP]);
    if Flag = 0 then
      Inc(IP, Offset)
    else
      Inc(IP);
  end;
end;

procedure TForth.WordIF;
var
  XT: Integer;
begin
  if not State then
    raise EForthError.Create('IF ONLY IN COMPILATION');
  XT := EnsureNamedPrimitive('0BRANCH', @WordZeroBranch);
  CompileAppend(TCell(XT));
  CompileAppend(0);
  PushDS(TCell(High(CompileList)));
end;

procedure TForth.WordTHEN;
var
  Pos: Integer;
begin
  if not State then
    raise EForthError.Create('THEN ONLY IN COMPILATION');
  Pos := Integer(PopDS);
  if (Pos < 0) or (Pos >= Length(CompileList)) then
    raise EForthError.Create('INVALID BRANCH TARGET');
  CompileList[Pos] := TCell(Length(CompileList) - Pos);
end;

procedure TForth.WordELSE;
var
  BranchXT, Pos1, Pos2: Integer;
begin
  if not State then
    raise EForthError.Create('ELSE ONLY IN COMPILATION');
  BranchXT := EnsureNamedPrimitive('BRANCH', @WordBranch);
  CompileAppend(TCell(BranchXT));
  CompileAppend(0);
  Pos2 := High(CompileList);
  Pos1 := Integer(PopDS);
  if (Pos1 < 0) or (Pos1 >= Length(CompileList)) then
    raise EForthError.Create('INVALID BRANCH TARGET');
  CompileList[Pos1] := TCell(Pos2 + 1 - Pos1);
  PushDS(TCell(Pos2));
end;

procedure TForth.WordBEGIN;
begin
  if not State then
    raise EForthError.Create('BEGIN ONLY IN COMPILATION');
  PushDS(TCell(Length(CompileList)));
end;

procedure TForth.WordUNTIL;
var
  Dest, XT: Integer;
begin
  if not State then
    raise EForthError.Create('UNTIL ONLY IN COMPILATION');
  Dest := Integer(PopDS);
  XT := EnsureNamedPrimitive('0BRANCH', @WordZeroBranch);
  CompileAppend(TCell(XT));
  CompileAppend(TCell(Dest - Length(CompileList)));
end;

procedure TForth.WordAGAIN;
var
  Dest, BranchXT: Integer;
begin
  if not State then
    raise EForthError.Create('AGAIN ONLY IN COMPILATION');
  Dest := Integer(PopDS);
  BranchXT := EnsureNamedPrimitive('BRANCH', @WordBranch);
  CompileAppend(TCell(BranchXT));
  CompileAppend(TCell(Dest - Length(CompileList)));
end;

procedure TForth.WordWHILE;
var
  Dest: TCell;
  XT, Orig: Integer;
begin
  if not State then
    raise EForthError.Create('WHILE ONLY IN COMPILATION');
  Dest := PopDS;
  XT := EnsureNamedPrimitive('0BRANCH', @WordZeroBranch);
  CompileAppend(TCell(XT));
  CompileAppend(0);
  Orig := High(CompileList);
  PushDS(Dest);
  PushDS(TCell(Orig));
end;

procedure TForth.WordREPEAT;
var
  Orig, Dest, BranchXT: Integer;
begin
  if not State then
    raise EForthError.Create('REPEAT ONLY IN COMPILATION');
  Orig := Integer(PopDS);
  Dest := Integer(PopDS);
  BranchXT := EnsureNamedPrimitive('BRANCH', @WordBranch);
  CompileAppend(TCell(BranchXT));
  CompileAppend(TCell(Dest - Length(CompileList)));
  if (Orig < 0) or (Orig >= Length(CompileList)) then
    raise EForthError.Create('INVALID BRANCH TARGET');
  CompileList[Orig] := TCell(Length(CompileList) - Orig);
end;

procedure TForth.WordDoRun;
var
  Start, Limit: TCell;
begin
  Start := PopDS;
  Limit := PopDS;
  PushRS(Limit);
  PushRS(Start);
end;

procedure TForth.WordLoopRun;
var
  Index, Limit: TCell;
  Offset: Integer;
begin
  Index := PopRS;
  Limit := PopRS;
  Index := WrapCell(Integer(Index) + 1);
  if Index < Limit then
  begin
    PushRS(Limit);
    PushRS(Index);
    if IP < Length(CurrentBody) then
    begin
      Offset := Integer(CurrentBody[IP]);
      Inc(IP, Offset);
    end;
  end
  else
    Inc(IP);
end;

procedure TForth.WordPlusLoopRun;
var
  Step, Index, Limit: TCell;
  Offset: Integer;
  Done: Boolean;
begin
  Step := PopDS;
  Index := PopRS;
  Limit := PopRS;
  Index := WrapCell(Integer(Index) + Integer(Step));
  Done := ((Step > 0) and (Index >= Limit)) or ((Step < 0) and (Index <= Limit));
  if Done then
    Inc(IP)
  else
  begin
    PushRS(Limit);
    PushRS(Index);
    if IP < Length(CurrentBody) then
    begin
      Offset := Integer(CurrentBody[IP]);
      Inc(IP, Offset);
    end;
  end;
end;

procedure TForth.WordLeaveRun;
begin
  if Length(RS) >= 2 then
    RS[High(RS)] := RS[High(RS) - 1];
end;

procedure TForth.WordDo;
var
  XT: Integer;
begin
  if not State then
    raise EForthError.Create('DO ONLY IN COMPILATION');
  XT := EnsureNamedPrimitive('(DO)', @WordDoRun);
  CompileAppend(TCell(XT));
  PushDS(TCell(Length(CompileList)));
end;

procedure TForth.WordLoop;
var
  Dest, XT: Integer;
begin
  if not State then
    raise EForthError.Create('LOOP ONLY IN COMPILATION');
  Dest := Integer(PopDS);
  XT := EnsureNamedPrimitive('(LOOP)', @WordLoopRun);
  CompileAppend(TCell(XT));
  CompileAppend(TCell(Dest - Length(CompileList)));
end;

procedure TForth.WordPlusLoop;
var
  Dest, XT: Integer;
begin
  if not State then
    raise EForthError.Create('+LOOP ONLY IN COMPILATION');
  Dest := Integer(PopDS);
  XT := EnsureNamedPrimitive('(+LOOP)', @WordPlusLoopRun);
  CompileAppend(TCell(XT));
  CompileAppend(TCell(Dest - Length(CompileList)));
end;

procedure TForth.WordI;
begin
  PushDS(PeekRS);
end;

procedure TForth.WordJ;
begin
  if Length(RS) < 4 then
    raise EForthError.Create('J: NOT ENOUGH LOOP LEVELS');
  PushDS(RS[High(RS) - 2]);
end;

procedure TForth.WordLeave;
var
  XT: Integer;
begin
  if not State then
    raise EForthError.Create('LEAVE ONLY IN COMPILATION');
  XT := EnsureNamedPrimitive('(LEAVE)', @WordLeaveRun);
  CompileAppend(TCell(XT));
end;

procedure TForth.WordUnloop;
begin
  if Length(RS) < 2 then
    raise EForthError.Create('RETURN STACK UNDERFLOW');
  PopRS;
  PopRS;
end;

function TForth.FormatCell(Val: TCell; ABase: Integer; IsSigned: Boolean): String;
const
  Digits = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';
var
  Base: Integer;
  Neg: Boolean;
  UVal: LongWord;
  Buf: String;
  Dig: Integer;
  I, J: Integer;
  Tmp: Char;
begin
  Base := ABase;
  if (Base < 2) or (Base > 36) then
    Base := 10;
  if Val = 0 then
  begin
    Result := '0';
    Exit;
  end;
  Neg := False;
  if IsSigned and (Val < 0) then
  begin
    Neg := True;
    UVal := LongWord(-Integer(Val));
  end
  else
    UVal := LongWord(Word(Val));
  Buf := '';
  while UVal > 0 do
  begin
    Dig := Integer(UVal mod LongWord(Base));
    Buf := Buf + Digits[Dig + 1];
    UVal := UVal div LongWord(Base);
  end;
  if Neg then
    Buf := Buf + '-';
  I := 1;
  J := Length(Buf);
  while I < J do
  begin
    Tmp := Buf[I];
    Buf[I] := Buf[J];
    Buf[J] := Tmp;
    Inc(I);
    Dec(J);
  end;
  Result := Buf;
end;

procedure TForth.WordDot;
begin
  EmitString(FormatCell(PopDS, GetBase, True) + ' ');
end;

procedure TForth.WordUDot;
begin
  EmitString(FormatCell(PopDS, GetBase, False) + ' ');
end;

procedure TForth.WordDotS;
var
  I: Integer;
  B: Integer;
begin
  B := GetBase;
  EmitString('<' + IntToStr(Length(DS)) + '>');
  for I := 0 to High(DS) do
    EmitString(' ' + FormatCell(DS[I], B, True));
  EmitString(' ');
end;

procedure TForth.WordPage;
begin
  EmitString(#27'[2J' + #27'[H');
end;

function TForth.ParseStringLiteral: String;
begin
  if (IN_PTR <= Length(TIB)) and (TIB[IN_PTR] = ' ') then
    Inc(IN_PTR);
  Result := '';
  while (IN_PTR <= Length(TIB)) and (TIB[IN_PTR] <> '"') do
  begin
    Result := Result + TIB[IN_PTR];
    Inc(IN_PTR);
  end;
  if (IN_PTR <= Length(TIB)) and (TIB[IN_PTR] = '"') then
    Inc(IN_PTR);
end;

procedure TForth.WordDotQuote;
var
  S: String;
  Addr, I, LitXT, TypeXT: Integer;
begin
  S := ParseStringLiteral;
  if State then
  begin
    Addr := Here;
    for I := 1 to Length(S) do
      CStore(Addr + I - 1, Byte(Ord(S[I])));
    Allot(Length(S));
    LitXT := EnsureLitXT;
    TypeXT := EnsureNamedPrimitive('TYPE', @WordType);
    CompileAppend(TCell(LitXT));
    CompileAppend(TCell(Addr));
    CompileAppend(TCell(LitXT));
    CompileAppend(TCell(Length(S)));
    CompileAppend(TCell(TypeXT));
  end
  else
    EmitString(S);
end;

procedure TForth.WordSQuote;
var
  S: String;
  Addr, I, LitXT: Integer;
begin
  S := ParseStringLiteral;
  Addr := Here;
  for I := 1 to Length(S) do
    CStore(Addr + I - 1, Byte(Ord(S[I])));
  Allot(Length(S));
  if State then
  begin
    LitXT := EnsureLitXT;
    CompileAppend(TCell(LitXT));
    CompileAppend(TCell(Addr));
    CompileAppend(TCell(LitXT));
    CompileAppend(TCell(Length(S)));
  end
  else
  begin
    PushDS(TCell(Addr));
    PushDS(TCell(Length(S)));
  end;
end;

procedure TForth.WordParen;
begin
  while (IN_PTR <= Length(TIB)) and (TIB[IN_PTR] <> ')') do
    Inc(IN_PTR);
  if (IN_PTR <= Length(TIB)) and (TIB[IN_PTR] = ')') then
    Inc(IN_PTR);
end;

procedure TForth.WordBackslash;
begin
  IN_PTR := Length(TIB) + 1;
end;

procedure TForth.InitPrimitives;
begin
  { Kernel registration grows as specs require. }
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
  WordInterpret;
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
