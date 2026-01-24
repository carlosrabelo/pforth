program test_runner;

{$mode objfpc}{$H+}

uses
  SysUtils, Classes, pforth_cli, pforth_types, pforth_engine;

var
  TotalTests: Integer = 0;
  PassedTests: Integer = 0;
  FailedTests: Integer = 0;

procedure AssertTrue(const Desc: String; Condition: Boolean);
begin
  Inc(TotalTests);
  if Condition then
  begin
    Inc(PassedTests);
    WriteLn('  [PASS] ', Desc);
  end
  else
  begin
    Inc(FailedTests);
    WriteLn('  [FAIL] ', Desc);
  end;
end;

procedure TestCLIArgDispatch;
var
  EmptyArgs, FileArgs: TStringArray;
  Cfg1, Cfg2: TCLIConfig;
begin
  WriteLn('Testing CLI argument dispatch...');
  SetLength(EmptyArgs, 0);
  Cfg1 := ParseArgs(EmptyArgs);
  AssertTrue('Empty args produces interactive mode', Cfg1.Interactive);
  AssertTrue('Empty args produces 0 file paths', Length(Cfg1.FilePaths) = 0);

  SetLength(FileArgs, 2);
  FileArgs[0] := 'demo1.fs';
  FileArgs[1] := 'demo2.fs';
  Cfg2 := ParseArgs(FileArgs);
  AssertTrue('File args produces non-interactive mode', not Cfg2.Interactive);
  AssertTrue('File args count is 2', Length(Cfg2.FilePaths) = 2);
  AssertTrue('File arg 0 is demo1.fs', Cfg2.FilePaths[0] = 'demo1.fs');
  AssertTrue('File arg 1 is demo2.fs', Cfg2.FilePaths[1] = 'demo2.fs');

  AssertTrue('RunCLI interactive returns 0', RunCLI(Cfg1) = 0);

  SetLength(FileArgs, 1);
  FileArgs[0] := '/no/such/pforth_missing_xyz.fs';
  Cfg2 := ParseArgs(FileArgs);
  AssertTrue('RunCLI missing file returns 1', RunCLI(Cfg2) = 1);
end;

procedure TestConsoleIOPrimitives;
var
  InStream, OutStream: TStringStream;
  F: TForth;
  I: Integer;
  UnderflowCaught: Boolean;
begin
  WriteLn('Testing Console I/O primitives...');
  InStream := TStringStream.Create('X');
  OutStream := TStringStream.Create('');
  F := TForth.Create(InStream, OutStream);
  try
    // KEY
    F.WordKey;
    AssertTrue('KEY reads X onto DS', F.DepthDS = 1);
    AssertTrue('KEY value is Ord(X)', F.PeekDS = Ord('X'));

    // EMIT
    F.WordEmit;
    AssertTrue('EMIT pops DS', F.DepthDS = 0);
    AssertTrue('EMIT writes X to output', OutStream.DataString = 'X');

    // KEY at EOF pushes 0
    F.WordKey;
    AssertTrue('KEY at EOF pushes 0', (F.DepthDS = 1) and (F.PopDS = 0));

    // CR, SPACE, SPACES
    OutStream.Position := 0;
    OutStream.Size := 0;
    F.WordCR;
    F.WordSpace;
    F.PushDS(3);
    F.WordSpaces;
    AssertTrue('CR SPACE SPACES outputs newline and 4 spaces', OutStream.DataString = #10'    ');

    // TYPE
    OutStream.Position := 0;
    OutStream.Size := 0;
    for I := 0 to 4 do
      F.Memory[1024 + I] := Byte(Ord('HELLO'[I + 1]));
    F.PushDS(1024);
    F.PushDS(5);
    F.WordType;
    AssertTrue('TYPE outputs HELLO', OutStream.DataString = 'HELLO');
    AssertTrue('TYPE leaves DS empty', F.DepthDS = 0);

    // BYE
    AssertTrue('Running is true before BYE', F.Running);
    F.WordBye;
    AssertTrue('Running is false after BYE', not F.Running);

    // Stack underflow raises exception
    UnderflowCaught := False;
    try
      F.PopDS;
    except
      on E: EForthError do
        UnderflowCaught := True;
    end;
    AssertTrue('PopDS underflow raises EForthError', UnderflowCaught);
  finally
    F.Free;
    InStream.Free;
    OutStream.Free;
  end;
end;

procedure TestLineReaderAndTIB;
var
  InStream, OutStream: TStringStream;
  F: TForth;
  I: Integer;
  Res: String;
  Ok: Boolean;
begin
  WriteLn('Testing Line reader and TIB...');
  InStream := TStringStream.Create('TEST LINE'#10);
  OutStream := TStringStream.Create('');
  F := TForth.Create(InStream, OutStream);
  try
    // EXPECT buffer at 2000, maxlen 20
    F.PushDS(2000);
    F.PushDS(20);
    F.WordExpect;

    Res := '';
    for I := 0 to 8 do
      Res := Res + Chr(F.Memory[2000 + I]);
    AssertTrue('EXPECT stored TEST LINE at 2000', Res = 'TEST LINE');

    // SPAN
    F.WordSpan;
    AssertTrue('SPAN pushes 1000', F.PopDS = 1000);
    AssertTrue('Memory at SPAN holds length 9', F.FetchCell(1000) = 9);
  finally
    F.Free;
    InStream.Free;
    OutStream.Free;
  end;

  // EXPECT truncates to maxlen
  InStream := TStringStream.Create('A VERY LONG STRING'#10);
  OutStream := TStringStream.Create('');
  F := TForth.Create(InStream, OutStream);
  try
    F.PushDS(3000);
    F.PushDS(5);
    F.WordExpect;

    Res := '';
    for I := 0 to 4 do
      Res := Res + Chr(F.Memory[3000 + I]);
    AssertTrue('EXPECT truncated to maxlen 5', Res = 'A VER');
    AssertTrue('Memory at SPAN holds length 5', F.FetchCell(1000) = 5);
  finally
    F.Free;
    InStream.Free;
    OutStream.Free;
  end;

  // ReadLine and SetLine
  InStream := TStringStream.Create('FIRST LINE'#13#10'SECOND LINE'#10);
  OutStream := TStringStream.Create('');
  F := TForth.Create(InStream, OutStream);
  try
    Ok := F.ReadLine(Res);
    AssertTrue('ReadLine 1 returns True', Ok);
    AssertTrue('ReadLine 1 matches FIRST LINE', Res = 'FIRST LINE');

    Ok := F.ReadLine(Res);
    AssertTrue('ReadLine 2 returns True', Ok);
    AssertTrue('ReadLine 2 matches SECOND LINE', Res = 'SECOND LINE');

    Ok := F.ReadLine(Res);
    AssertTrue('ReadLine at EOF returns False', not Ok);

    F.SetLine('LINE FOR TIB');
    AssertTrue('SetLine sets TIB', F.TIB = 'LINE FOR TIB');
    AssertTrue('SetLine resets IN_PTR', F.IN_PTR = 1);
  finally
    F.Free;
    InStream.Free;
    OutStream.Free;
  end;
end;

procedure TestMemoryModelAndEndianness;
var
  F: TForth;
  Raised: Boolean;
begin
  WriteLn('--- TestMemoryModelAndEndianness ---');
  F := TForth.Create(nil, TMemoryStream.Create);
  try
    F.StoreCell(1024, $1234);
    AssertTrue('little-endian low byte 0x34', F.Memory[1024] = $34);
    AssertTrue('little-endian high byte 0x12', F.Memory[1025] = $12);
    AssertTrue('FetchCell 0x1234', F.FetchCell(1024) = $1234);

    F.StoreCell(2000, -1);
    AssertTrue('StoreCell -1 low FF', F.Memory[2000] = $FF);
    AssertTrue('StoreCell -1 high FF', F.Memory[2001] = $FF);
    AssertTrue('FetchCell -1', F.FetchCell(2000) = -1);

    Raised := False;
    try
      F.CFetch(65536);
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('byte OOR message', E.Message = 'OUT OF MEMORY RANGE');
      end;
    end;
    AssertTrue('CFetch 65536 raises', Raised);

    Raised := False;
    try
      F.StoreCell(65535, 1);
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('cell OOR message', E.Message = 'OUT OF MEMORY RANGE');
      end;
    end;
    AssertTrue('StoreCell 65535 raises', Raised);

    Raised := False;
    try
      F.PopRS;
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('RS underflow message', E.Message = 'RETURN STACK UNDERFLOW');
      end;
    end;
    AssertTrue('PopRS underflow raises', Raised);
  finally
    F.OutputStream.Free;
    F.Free;
  end;
end;

procedure TestDoubleStackWords;
var
  F: TForth;
  OutStream: TMemoryStream;
begin
  WriteLn('--- TestDoubleStackWords ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.PushDS(1);
    F.PushDS(2);
    F.WordTwoDup;
    AssertTrue('2DUP d', F.PopDS = 2);
    AssertTrue('2DUP c', F.PopDS = 1);
    AssertTrue('2DUP b', F.PopDS = 2);
    AssertTrue('2DUP a', F.PopDS = 1);

    F.PushDS(3);
    F.PushDS(4);
    F.WordTwoDrop;
    AssertTrue('2DROP empty', F.DepthDS = 0);

    F.PushDS(1);
    F.PushDS(2);
    F.PushDS(3);
    F.PushDS(4);
    F.WordTwoSwap;
    AssertTrue('2SWAP top 2', F.PopDS = 2);
    AssertTrue('2SWAP 1', F.PopDS = 1);
    AssertTrue('2SWAP 4', F.PopDS = 4);
    AssertTrue('2SWAP 3', F.PopDS = 3);

    F.PushDS(1);
    F.PushDS(2);
    F.PushDS(3);
    F.PushDS(4);
    F.WordTwoOver;
    AssertTrue('2OVER top 2', F.PopDS = 2);
    AssertTrue('2OVER 1', F.PopDS = 1);
    AssertTrue('2OVER 4', F.PopDS = 4);
    AssertTrue('2OVER 3', F.PopDS = 3);
    AssertTrue('2OVER 2', F.PopDS = 2);
    AssertTrue('2OVER bot 1', F.PopDS = 1);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestScalingHelpers;
var
  F: TForth;
  OutStream: TMemoryStream;
begin
  WriteLn('--- TestScalingHelpers ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.PushDS(10);
    F.WordOnePlus;
    AssertTrue('1+', F.PopDS = 11);
    F.PushDS(10);
    F.WordOneMinus;
    AssertTrue('1-', F.PopDS = 9);
    F.PushDS(10);
    F.WordTwoPlus;
    AssertTrue('2+', F.PopDS = 12);
    F.PushDS(10);
    F.WordTwoMinus;
    AssertTrue('2-', F.PopDS = 8);
    F.PushDS(5);
    F.WordTwoStar;
    AssertTrue('2*', F.PopDS = 10);
    F.PushDS(-4);
    F.WordTwoSlash;
    AssertTrue('2/ neg', F.PopDS = -2);
    F.PushDS(-7);
    F.WordAbs;
    AssertTrue('ABS', F.PopDS = 7);
    F.PushDS(3);
    F.PushDS(8);
    F.WordMin;
    AssertTrue('MIN', F.PopDS = 3);
    F.PushDS(3);
    F.PushDS(8);
    F.WordMax;
    AssertTrue('MAX', F.PopDS = 8);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestBitwiseLogic;
var
  F: TForth;
  OutStream: TMemoryStream;
begin
  WriteLn('--- TestBitwiseLogic ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.PushDS($0F0F);
    F.PushDS($00FF);
    F.WordAnd;
    AssertTrue('AND', F.PopDS = $000F);
    F.PushDS($0F00);
    F.PushDS($00F0);
    F.WordOr;
    AssertTrue('OR', F.PopDS = $0FF0);
    F.PushDS($0FF0);
    F.PushDS($00FF);
    F.WordXor;
    AssertTrue('XOR', F.PopDS = $0F0F);
    F.PushDS(0);
    F.WordInvert;
    AssertTrue('INVERT 0', F.PopDS = -1);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestDictionaryPointerComma;
var
  F: TForth;
  OutStream: TMemoryStream;
begin
  WriteLn('--- TestDictionaryPointerComma ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.WordHere;
    AssertTrue('HERE starts 1024', F.PopDS = 1024);
    F.WordDP;
    AssertTrue('DP addr 1002', F.PopDS = 1002);
    F.WordPAD;
    AssertTrue('PAD is HERE+68', F.PopDS = 1024 + 68);

    F.PushDS(10);
    F.WordAllot;
    F.WordHere;
    AssertTrue('ALLOT 10', F.PopDS = 1034);

    F.PushDS($1234);
    F.WordComma;
    AssertTrue('comma low', F.Memory[1034] = $34);
    AssertTrue('comma high', F.Memory[1035] = $12);
    F.WordHere;
    AssertTrue('HERE after ,', F.PopDS = 1036);

    F.PushDS(Ord('Z'));
    F.WordCComma;
    AssertTrue('C, byte', F.Memory[1036] = Ord('Z'));
    F.WordHere;
    AssertTrue('HERE after C,', F.PopDS = 1037);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestExecuteInnerInterpreter;
var
  F: TForth;
  OutStream: TMemoryStream;
  Body: TCellStack;
  Raised: Boolean;
  XT, DupXT, ExitXT: Integer;
begin
  WriteLn('--- TestExecuteInnerInterpreter ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.DefinePrimitive('DUP', @F.WordDup);
    F.DefinePrimitive('EXIT', @F.WordExit);
    SetLength(Body, 1);
    Body[0] := 99;
    XT := F.DefineWord('NINETY', wkConstant, nil, Body, False);
    F.PushDS(TCell(XT));
    F.WordExecute;
    AssertTrue('EXECUTE constant', F.PopDS = 99);

    Raised := False;
    try
      F.ExecuteWord(9999);
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('bad XT msg', Pos('INVALID EXECUTION TOKEN:', E.Message) = 1);
      end;
    end;
    AssertTrue('bad XT raises', Raised);

    F.FindXT('DUP', DupXT);
    F.FindXT('EXIT', ExitXT);
    SetLength(Body, 3);
    Body[0] := TCell(DupXT);
    Body[1] := TCell(DupXT);
    Body[2] := TCell(ExitXT);
    XT := F.DefineWord('DD', wkColon, nil, Body, False);
    F.PushDS(7);
    F.ExecuteWord(XT);
    AssertTrue('colon depth', F.DepthDS = 3);
    AssertTrue('colon top', F.PopDS = 7);
    AssertTrue('colon mid', F.PopDS = 7);
    AssertTrue('colon bot', F.PopDS = 7);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestWordNumberBase;
var
  F: TForth;
  OutStream: TMemoryStream;
  Num: TCell;
begin
  WriteLn('--- TestWordNumberBase ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.WordBase;
    AssertTrue('BASE addr', F.PopDS = 1004);
    AssertTrue('ParseNumber 42', F.ParseNumber('42', Num) and (Num = 42));
    AssertTrue('ParseNumber $10', F.ParseNumber('$10', Num) and (Num = 16));
    AssertTrue('ParseNumber %101', F.ParseNumber('%101', Num) and (Num = 5));
    AssertTrue('ParseNumber -3', F.ParseNumber('-3', Num) and (Num = -3));
    F.StoreCell(1004, 16);
    AssertTrue('base 16 FF', F.ParseNumber('FF', Num) and (Num = 255));
    F.StoreCell(1004, 10);

    F.SetLine('HELLO');
    F.PushDS(Ord(' '));
    F.WordWord;
    AssertTrue('WORD addr HERE', F.PeekDS = 1024);
    AssertTrue('WORD count 5', F.CFetch(1024) = 5);
    AssertTrue('WORD H', F.CFetch(1025) = Ord('H'));
    AssertTrue('WORD trailing space', F.CFetch(1030) = Ord(' '));

    F.PushDS(1024);
    F.WordNumber;
    AssertTrue('NUMBER fail flag', F.PopDS = 0);
    AssertTrue('NUMBER fail n', F.PopDS = 0);

    F.CStore(2000, 2);
    F.CStore(2001, Ord('4'));
    F.CStore(2002, Ord('2'));
    F.PushDS(2000);
    F.WordNumber;
    AssertTrue('NUMBER ok flag', F.PopDS = -1);
    AssertTrue('NUMBER ok n', F.PopDS = 42);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestInterpretAndLit;
var
  F: TForth;
  OutStream: TMemoryStream;
  Raised: Boolean;
begin
  WriteLn('--- TestInterpretAndLit ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.DefinePrimitive('DUP', @F.WordDup);
    F.InterpretLine('5 DUP');
    AssertTrue('interpret 5 DUP depth', F.DepthDS = 2);
    AssertTrue('interpret top', F.PopDS = 5);
    AssertTrue('interpret second', F.PopDS = 5);

    Raised := False;
    try
      F.InterpretLine('NOSUCHWORD');
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('unknown msg', E.Message = 'UNKNOWN WORD: NOSUCHWORD');
      end;
    end;
    AssertTrue('unknown raises', Raised);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestCompilerStateWords;
var
  F: TForth;
  OutStream: TMemoryStream;
begin
  WriteLn('--- TestCompilerStateWords ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.DefinePrimitive('DUP', @F.WordDup);
    F.DefinePrimitive('LIT', @F.WordLIT);
    F.WordState;
    AssertTrue('STATE interpreting', F.PopDS = 0);
    F.WordRightBracket;
    F.WordState;
    AssertTrue('STATE compiling', F.PopDS = -1);
    F.PushDS(9);
    F.WordLiteral;
    AssertTrue('LITERAL compile len', Length(F.CompileList) = 2);
    F.WordLeftBracket;
    F.WordState;
    AssertTrue('STATE after [', F.PopDS = 0);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure RegisterControlWords(F: TForth);
begin
  F.DefinePrimitive('BRANCH', @F.WordBranch);
  F.DefinePrimitive('0BRANCH', @F.WordZeroBranch);
  F.DefinePrimitive('DUP', @F.WordDup);
  F.DefinePrimitive('DROP', @F.WordDrop);
  F.DefinePrimitive('+', @F.WordPlus);
  F.DefinePrimitive('1+', @F.WordOnePlus);
  F.DefinePrimitive('=', @F.WordEquals);
  F.DefinePrimitive('EXIT', @F.WordExit);
  F.DefinePrimitive('LIT', @F.WordLIT);
  F.DefinePrimitive(':', @F.WordColon);
  F.DefineImmediate(';', @F.WordSemicolon);
  F.DefineImmediate('IF', @F.WordIF);
  F.DefineImmediate('ELSE', @F.WordELSE);
  F.DefineImmediate('THEN', @F.WordTHEN);
  F.DefineImmediate('BEGIN', @F.WordBEGIN);
  F.DefineImmediate('UNTIL', @F.WordUNTIL);
  F.DefineImmediate('AGAIN', @F.WordAGAIN);
  F.DefineImmediate('WHILE', @F.WordWHILE);
  F.DefineImmediate('REPEAT', @F.WordREPEAT);
  F.DefineImmediate('DO', @F.WordDo);
  F.DefineImmediate('LOOP', @F.WordLoop);
  F.DefineImmediate('+LOOP', @F.WordPlusLoop);
  F.DefinePrimitive('I', @F.WordI);
  F.DefinePrimitive('J', @F.WordJ);
  F.DefineImmediate('LEAVE', @F.WordLeave);
  F.DefinePrimitive('UNLOOP', @F.WordUnloop);
end;

procedure TestBranchRuntime;
var
  F: TForth;
  OutStream: TMemoryStream;
  Body: TCellStack;
  XT, LitXT, ZBranchXT, ExitXT: Integer;
begin
  WriteLn('--- TestBranchRuntime ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    LitXT := F.DefinePrimitive('LIT', @F.WordLIT);
    ZBranchXT := F.DefinePrimitive('0BRANCH', @F.WordZeroBranch);
    ExitXT := F.DefinePrimitive('EXIT', @F.WordExit);
    { LIT 0  0BRANCH 4  LIT 99 EXIT  LIT 42 EXIT
      offset at index 3; IP+=4 -> index 7 = LIT 42 }
    SetLength(Body, 10);
    Body[0] := TCell(LitXT);
    Body[1] := 0;
    Body[2] := TCell(ZBranchXT);
    Body[3] := 4;
    Body[4] := TCell(LitXT);
    Body[5] := 99;
    Body[6] := TCell(ExitXT);
    Body[7] := TCell(LitXT);
    Body[8] := 42;
    Body[9] := TCell(ExitXT);
    XT := F.DefineWord('ZB', wkColon, nil, Body, False);
    F.ExecuteWord(XT);
    AssertTrue('0BRANCH taken leaves 42', F.PopDS = 42);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestIfElseThen;
var
  F: TForth;
  OutStream: TMemoryStream;
  XT: Integer;
begin
  WriteLn('--- TestIfElseThen ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    RegisterControlWords(F);
    F.InterpretLine(': TIF IF 1 ELSE 2 THEN ;');
    F.FindXT('TIF', XT);
    F.PushDS(-1);
    F.ExecuteWord(XT);
    AssertTrue('IF true', F.PopDS = 1);
    F.PushDS(0);
    F.ExecuteWord(XT);
    AssertTrue('IF false', F.PopDS = 2);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestBeginLoops;
var
  F: TForth;
  OutStream: TMemoryStream;
  XT: Integer;
begin
  WriteLn('--- TestBeginLoops ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    RegisterControlWords(F);
    F.InterpretLine(': COUNTDOWN BEGIN DUP 1+ DUP 3 = UNTIL ;');
    F.FindXT('COUNTDOWN', XT);
    F.PushDS(1);
    F.ExecuteWord(XT);
    AssertTrue('BEGIN UNTIL top 3', F.PopDS = 3);
    AssertTrue('BEGIN UNTIL mid 2', F.PopDS = 2);
    AssertTrue('BEGIN UNTIL bot 1', F.PopDS = 1);

    F.DefinePrimitive('<', @F.WordLess);
    F.InterpretLine(': SUM3 0 BEGIN DUP 3 < WHILE 1+ REPEAT ;');
    F.FindXT('SUM3', XT);
    F.ExecuteWord(XT);
    AssertTrue('WHILE REPEAT', F.PopDS = 3);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestRequireAndCore;
var
  F: TForth;
  OutStream: TMemoryStream;
  XT: Integer;
  Raised: Boolean;
  W: TForthWord;
begin
  WriteLn('--- TestRequireAndCore ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    AssertTrue('CORE loaded at cold start', F.FindXT('NIP', XT));
    AssertTrue('NIP is colon', F.Words[XT].Kind = wkColon);
    AssertTrue('TUCK is colon', F.Lookup('TUCK', W) and (W.Kind = wkColon));
    AssertTrue('ROT is colon', F.Lookup('ROT', W) and (W.Kind = wkColon));

    F.InterpretLine('10 20 NIP');
    AssertTrue('CORE NIP', F.PopDS = 20);

    Raised := False;
    try
      F.SetLine('');
      F.WordRequire;
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('MODULE NAME EXPECTED', E.Message = 'MODULE NAME EXPECTED');
      end;
    end;
    AssertTrue('empty #REQUIRE raises', Raised);

    Raised := False;
    try
      F.RequireModule('NOSUCHMODULE');
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('MODULE NOT FOUND', E.Message = 'MODULE NOT FOUND: NOSUCHMODULE');
      end;
    end;
    AssertTrue('missing module raises', Raised);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestEvaluateAndInclude;
var
  F: TForth;
  OutStream: TMemoryStream;
  Raised: Boolean;
  Path: String;
  SL: TStringList;
  Addr: Integer;
  I: Integer;
begin
  WriteLn('--- TestEvaluateAndInclude ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.DefinePrimitive('DUP', @F.WordDup);
    F.DefinePrimitive('EVALUATE', @F.WordEvaluate);
    F.DefinePrimitive('INCLUDE', @F.WordInclude);
    F.DefinePrimitive('INCLUDED', @F.WordIncluded);

    Addr := 4000;
    F.CStore(Addr, Ord('5'));
    F.CStore(Addr + 1, Ord(' '));
    F.CStore(Addr + 2, Ord('D'));
    F.CStore(Addr + 3, Ord('U'));
    F.CStore(Addr + 4, Ord('P'));
    F.PushDS(TCell(Addr));
    F.PushDS(5);
    F.WordEvaluate;
    AssertTrue('EVALUATE depth', F.DepthDS = 2);
    AssertTrue('EVALUATE value', F.PopDS = 5);
    F.PopDS;

    Raised := False;
    try
      F.SetLine('');
      F.WordInclude;
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('INCLUDE empty msg', E.Message = 'FILE NAME EXPECTED');
      end;
    end;
    AssertTrue('INCLUDE empty raises', Raised);

    Raised := False;
    try
      F.LoadFile('/no/such/pforth-file.fs');
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('CANNOT OPEN msg', Pos('CANNOT OPEN:', E.Message) = 1);
      end;
    end;
    AssertTrue('CANNOT OPEN raises', Raised);

    Path := GetTempDir + 'pforth_incl_test.fs';
    SL := TStringList.Create;
    try
      SL.Text := '7 DUP' + LineEnding;
      SL.SaveToFile(Path);
      F.InterpretLine('INCLUDE ' + Path);
      AssertTrue('INCLUDE file top', F.PopDS = 7);
      AssertTrue('INCLUDE file second', F.PopDS = 7);

      Addr := 5000;
      for I := 1 to Length(Path) do
        F.CStore(Addr + I - 1, Byte(Ord(Path[I])));
      F.PushDS(TCell(Addr));
      F.PushDS(TCell(Length(Path)));
      F.WordIncluded;
      AssertTrue('INCLUDED top', F.PopDS = 7);
      F.PopDS;
    finally
      SL.Free;
      if FileExists(Path) then
        DeleteFile(Path);
    end;
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestStringsAndComments;
var
  F: TForth;
  OutStream: TMemoryStream;
  OutText: String;
  Addr, Len: TCell;
begin
  WriteLn('--- TestStringsAndComments ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.DefineImmediate('."', @F.WordDotQuote);
    F.DefineImmediate('S"', @F.WordSQuote);
    F.DefineImmediate('(', @F.WordParen);
    F.DefineImmediate('\', @F.WordBackslash);
    F.DefinePrimitive('TYPE', @F.WordType);
    F.DefinePrimitive('DROP', @F.WordDrop);

    F.InterpretLine('." HI"');
    SetLength(OutText, OutStream.Size);
    if OutStream.Size > 0 then
    begin
      OutStream.Position := 0;
      OutStream.Read(OutText[1], OutStream.Size);
    end;
    AssertTrue('." prints HI', OutText = 'HI');

    F.InterpretLine('S" AB"');
    Len := F.PopDS;
    Addr := F.PopDS;
    AssertTrue('S" length 2', Len = 2);
    AssertTrue('S" bytes', (F.CFetch(Integer(Addr)) = Ord('A')) and
      (F.CFetch(Integer(Addr) + 1) = Ord('B')));

    F.PushDS(99);
    F.InterpretLine('( ignored )');
    AssertTrue('( leaves stack', F.PopDS = 99);

    F.PushDS(7);
    F.InterpretLine('\ DROP 1');
    AssertTrue('\ comments rest', F.PopDS = 7);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestNumericOutput;
var
  F: TForth;
  OutStream: TMemoryStream;
  OutText: String;
begin
  WriteLn('--- TestNumericOutput ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.PushDS(42);
    F.WordDot;
    SetLength(OutText, OutStream.Size);
    if OutStream.Size > 0 then
    begin
      OutStream.Position := 0;
      OutStream.Read(OutText[1], OutStream.Size);
    end;
    AssertTrue('. prints 42 ', OutText = '42 ');

    OutStream.Clear;
    F.PushDS(-1);
    F.WordUDot;
    SetLength(OutText, OutStream.Size);
    if OutStream.Size > 0 then
    begin
      OutStream.Position := 0;
      OutStream.Read(OutText[1], OutStream.Size);
    end;
    AssertTrue('U. prints 65535 ', OutText = '65535 ');

    OutStream.Clear;
    F.PushDS(3);
    F.PushDS(5);
    F.WordDotS;
    SetLength(OutText, OutStream.Size);
    if OutStream.Size > 0 then
    begin
      OutStream.Position := 0;
      OutStream.Read(OutText[1], OutStream.Size);
    end;
    AssertTrue('.S picture', OutText = '<2> 3 5 ');
    AssertTrue('.S leaves stack', (F.DepthDS = 2) and (F.PopDS = 5) and (F.PopDS = 3));

    OutStream.Clear;
    F.WordPage;
    SetLength(OutText, OutStream.Size);
    if OutStream.Size > 0 then
    begin
      OutStream.Position := 0;
      OutStream.Read(OutText[1], OutStream.Size);
    end;
    AssertTrue('PAGE clears screen', OutText = #27'[2J' + #27'[H');
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestDoLoop;
var
  F: TForth;
  OutStream: TMemoryStream;
  XT: Integer;
begin
  WriteLn('--- TestDoLoop ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    RegisterControlWords(F);
    F.InterpretLine(': SUM10 0 10 0 DO I + LOOP ;');
    F.FindXT('SUM10', XT);
    F.ExecuteWord(XT);
    AssertTrue('DO LOOP sum 0..9', F.PopDS = 45);

    F.InterpretLine(': OUTER 0 2 0 DO 2 0 DO J + LOOP LOOP ;');
    F.FindXT('OUTER', XT);
    F.ExecuteWord(XT);
    { J=0 twice + J=1 twice = 0+0+1+1 = 2 }
    AssertTrue('J nested sum', F.PopDS = 2);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestColonDefinitions;
var
  F: TForth;
  OutStream: TMemoryStream;
  XT: Integer;
begin
  WriteLn('--- TestColonDefinitions ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.DefinePrimitive('DUP', @F.WordDup);
    F.DefinePrimitive('EXIT', @F.WordExit);
    F.DefinePrimitive('+', @F.WordPlus);
    F.DefineImmediate(';', @F.WordSemicolon);
    F.DefinePrimitive(':', @F.WordColon);
    F.DefinePrimitive('IMMEDIATE', @F.WordImmediate);
    F.DefineImmediate('RECURSE', @F.WordRecurse);

    F.InterpretLine(': DOUBLE DUP + ;');
    AssertTrue('colon FindXT', F.FindXT('DOUBLE', XT));
    F.PushDS(6);
    F.ExecuteWord(XT);
    AssertTrue('DOUBLE 6', F.PopDS = 12);

    F.InterpretLine(': MARKER ; IMMEDIATE');
    F.FindXT('MARKER', XT);
    AssertTrue('IMMEDIATE flag', F.Words[XT].Immediate);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestDefiningWords;
var
  F: TForth;
  OutStream: TMemoryStream;
  XT: Integer;
  Raised: Boolean;
begin
  WriteLn('--- TestDefiningWords ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.SetLine('WIDGET');
    F.WordCreate;
    AssertTrue('CREATE FindXT', F.FindXT('WIDGET', XT));
    F.ExecuteWord(XT);
    AssertTrue('CREATE pushes PFA', F.PopDS = F.Words[XT].PFA);

    F.PushDS(42);
    F.SetLine('ANSWER');
    F.WordConstant;
    F.FindXT('ANSWER', XT);
    F.ExecuteWord(XT);
    AssertTrue('CONSTANT value', F.PopDS = 42);

    F.SetLine('COUNTER');
    F.WordVariable;
    F.FindXT('COUNTER', XT);
    F.ExecuteWord(XT);
    AssertTrue('VARIABLE addr', F.PopDS = F.Words[XT].PFA);
    AssertTrue('VARIABLE cell 0', F.FetchCell(F.Words[XT].PFA) = 0);

    Raised := False;
    try
      F.SetLine('');
      F.WordCreate;
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('CREATE empty msg', E.Message = 'WORD NAME EXPECTED');
      end;
    end;
    AssertTrue('CREATE empty raises', Raised);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestDictionaryFindTickBody;
var
  F: TForth;
  OutStream: TMemoryStream;
  XT: Integer;
  Raised: Boolean;
begin
  WriteLn('--- TestDictionaryFindTickBody ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.DefinePrimitive('DUP', @F.WordDup);
    F.DefineImmediate('IF', @F.WordDrop);
    AssertTrue('FindXT DUP', F.FindXT('dup', XT));
    AssertTrue('XT is latest DUP', XT = F.Latest - 1);

    F.CStore(3000, 3);
    F.CStore(3001, Ord('D'));
    F.CStore(3002, Ord('U'));
    F.CStore(3003, Ord('P'));
    F.PushDS(3000);
    F.WordFind;
    AssertTrue('FIND flag -1', F.PopDS = -1);
    AssertTrue('FIND xt', F.PopDS = TCell(XT));

    F.CStore(3010, 2);
    F.CStore(3011, Ord('I'));
    F.CStore(3012, Ord('F'));
    F.PushDS(3010);
    F.WordFind;
    AssertTrue('FIND immediate 1', F.PopDS = 1);
    F.PopDS;

    F.CStore(3020, 3);
    F.CStore(3021, Ord('F'));
    F.CStore(3022, Ord('O'));
    F.CStore(3023, Ord('O'));
    F.PushDS(3020);
    F.WordFind;
    AssertTrue('FIND miss flag 0', F.PopDS = 0);
    AssertTrue('FIND miss addr', F.PopDS = 3020);

    F.SetLine('DUP');
    F.WordTick;
    AssertTrue(''' DUP xt', F.PopDS = TCell(XT));

    Raised := False;
    try
      F.SetLine('');
      F.WordTick;
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('tick empty msg', E.Message = 'WORD NAME EXPECTED');
      end;
    end;
    AssertTrue('tick empty raises', Raised);

    Raised := False;
    try
      F.SetLine('NOSUCH');
      F.WordTick;
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('tick missing msg', E.Message = 'WORD NOT FOUND: NOSUCH');
      end;
    end;
    AssertTrue('tick missing raises', Raised);

    F.PushDS(0);
    F.WordToBody;
    AssertTrue('>BODY uses PFA', F.PopDS = F.Words[0].PFA);

    F.PushDS(-5);
    F.WordToBody;
    AssertTrue('>BODY fallback xt+2', F.PopDS = -3);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestComparisonFlags;
var
  F: TForth;
  OutStream: TMemoryStream;
begin
  WriteLn('--- TestComparisonFlags ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.PushDS(5);
    F.PushDS(5);
    F.WordEquals;
    AssertTrue('=', F.PopDS = -1);
    F.PushDS(5);
    F.PushDS(6);
    F.WordNotEquals;
    AssertTrue('<>', F.PopDS = -1);
    F.PushDS(3);
    F.PushDS(8);
    F.WordLess;
    AssertTrue('<', F.PopDS = -1);
    F.PushDS(8);
    F.PushDS(3);
    F.WordGreater;
    AssertTrue('>', F.PopDS = -1);
    F.PushDS(3);
    F.PushDS(3);
    F.WordLessEqual;
    AssertTrue('<=', F.PopDS = -1);
    F.PushDS(3);
    F.PushDS(3);
    F.WordGreaterEqual;
    AssertTrue('>=', F.PopDS = -1);
    F.PushDS(0);
    F.WordZeroEquals;
    AssertTrue('0=', F.PopDS = -1);
    F.PushDS(1);
    F.WordZeroNotEquals;
    AssertTrue('0<>', F.PopDS = -1);
    F.PushDS(-1);
    F.WordZeroLess;
    AssertTrue('0<', F.PopDS = -1);
    F.PushDS(1);
    F.WordZeroGreater;
    AssertTrue('0>', F.PopDS = -1);
    F.PushDS(-1);
    F.PushDS(1);
    F.WordULess;
    AssertTrue('U< high-bit', F.PopDS = 0);
    F.PushDS(-1);
    F.PushDS(1);
    F.WordUGreater;
    AssertTrue('U> high-bit', F.PopDS = -1);
    F.PushDS(-1);
    F.PushDS(-1);
    F.WordULessEqual;
    AssertTrue('U<=', F.PopDS = -1);
    F.PushDS(1);
    F.PushDS(-1);
    F.WordUGreaterEqual;
    AssertTrue('U>=', F.PopDS = 0);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestSignedArithmetic;
var
  F: TForth;
  OutStream: TMemoryStream;
  Raised: Boolean;
begin
  WriteLn('--- TestSignedArithmetic ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.PushDS(2);
    F.PushDS(3);
    F.WordPlus;
    AssertTrue('2 3 +', F.PopDS = 5);

    F.PushDS(10);
    F.PushDS(4);
    F.WordMinus;
    AssertTrue('10 4 -', F.PopDS = 6);

    F.PushDS(6);
    F.PushDS(7);
    F.WordMultiply;
    AssertTrue('6 7 *', F.PopDS = 42);

    F.PushDS(14);
    F.PushDS(3);
    F.WordSlashMod;
    AssertTrue('/MOD quot 4', F.PopDS = 4);
    AssertTrue('/MOD rem 2', F.PopDS = 2);

    F.PushDS(14);
    F.PushDS(3);
    F.WordSlash;
    AssertTrue('14 3 /', F.PopDS = 4);

    F.PushDS(14);
    F.PushDS(3);
    F.WordMod;
    AssertTrue('14 3 MOD', F.PopDS = 2);

    F.PushDS(5);
    F.WordNegate;
    AssertTrue('NEGATE 5', F.PopDS = -5);

    Raised := False;
    try
      F.PushDS(1);
      F.PushDS(0);
      F.WordSlashMod;
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('/MOD zero msg', E.Message = 'DIVISION BY ZERO');
      end;
    end;
    AssertTrue('/MOD zero raises', Raised);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestReturnStackWords;
var
  F: TForth;
  OutStream: TMemoryStream;
begin
  WriteLn('--- TestReturnStackWords ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.PushDS(42);
    F.WordToR;
    AssertTrue('>R clears DS', F.DepthDS = 0);
    AssertTrue('>R depth RS', F.DepthRS = 1);
    F.WordRFetch;
    AssertTrue('R@ copies', F.PopDS = 42);
    AssertTrue('R@ leaves RS', F.DepthRS = 1);
    F.WordRFrom;
    AssertTrue('R> value', F.PopDS = 42);
    AssertTrue('R> clears RS', F.DepthRS = 0);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestShuffleStackWords;
var
  F: TForth;
  OutStream: TMemoryStream;
begin
  WriteLn('--- TestShuffleStackWords ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.PushDS(0);
    F.WordQDup;
    AssertTrue('?DUP zero stays single', F.DepthDS = 1);
    AssertTrue('?DUP zero value', F.PopDS = 0);
    F.PushDS(5);
    F.WordQDup;
    AssertTrue('?DUP non-zero depth', F.DepthDS = 2);
    AssertTrue('?DUP non-zero top', F.PopDS = 5);
    AssertTrue('?DUP non-zero second', F.PopDS = 5);

    F.PushDS(10);
    F.PushDS(20);
    F.PushDS(30);
    F.WordRot;
    AssertTrue('ROT top 10', F.PopDS = 10);
    AssertTrue('ROT mid 30', F.PopDS = 30);
    AssertTrue('ROT bot 20', F.PopDS = 20);

    F.PushDS(10);
    F.PushDS(20);
    F.PushDS(30);
    F.WordNRot;
    AssertTrue('-ROT top 20', F.PopDS = 20);
    AssertTrue('-ROT mid 10', F.PopDS = 10);
    AssertTrue('-ROT bot 30', F.PopDS = 30);

    F.PushDS(10);
    F.PushDS(20);
    F.WordNip;
    AssertTrue('NIP leaves 20', F.PopDS = 20);

    F.PushDS(10);
    F.PushDS(20);
    F.WordTuck;
    AssertTrue('TUCK top 20', F.PopDS = 20);
    AssertTrue('TUCK mid 10', F.PopDS = 10);
    AssertTrue('TUCK bot 20', F.PopDS = 20);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestBasicStackWords;
var
  F: TForth;
  OutStream: TMemoryStream;
  Raised: Boolean;
begin
  WriteLn('--- TestBasicStackWords ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.PushDS(7);
    F.WordDup;
    AssertTrue('DUP top', F.PopDS = 7);
    AssertTrue('DUP second', F.PopDS = 7);

    F.PushDS(1);
    F.PushDS(2);
    F.WordDrop;
    AssertTrue('DROP leaves 1', F.PopDS = 1);

    F.PushDS(10);
    F.PushDS(20);
    F.WordSwap;
    AssertTrue('SWAP top 10', F.PopDS = 10);
    AssertTrue('SWAP second 20', F.PopDS = 20);

    F.PushDS(1);
    F.PushDS(2);
    F.WordOver;
    AssertTrue('OVER top 1', F.PopDS = 1);
    AssertTrue('OVER mid 2', F.PopDS = 2);
    AssertTrue('OVER bot 1', F.PopDS = 1);

    Raised := False;
    try
      F.WordDup;
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('DUP underflow msg', E.Message = 'DATA STACK UNDERFLOW');
      end;
    end;
    AssertTrue('DUP underflow raises', Raised);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestMemoryBlockOperations;
var
  F: TForth;
  OutStream: TMemoryStream;
  I: Integer;
  Ok: Boolean;
begin
  WriteLn('--- TestMemoryBlockOperations ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.PushDS(4000);
    F.PushDS(5);
    F.PushDS($AB);
    F.WordFill;
    Ok := True;
    for I := 0 to 4 do
      if F.CFetch(4000 + I) <> $AB then
        Ok := False;
    AssertTrue('FILL writes five bytes', Ok);
    F.PushDS(4000);
    F.PushDS(0);
    F.PushDS($FF);
    F.WordFill;
    AssertTrue('FILL len 0 leaves first byte', F.CFetch(4000) = $AB);

    F.PushDS(4000);
    F.PushDS(3);
    F.WordErase;
    AssertTrue('ERASE clears byte 0', F.CFetch(4000) = 0);
    AssertTrue('ERASE clears byte 2', F.CFetch(4002) = 0);
    AssertTrue('ERASE leaves byte 3', F.CFetch(4003) = $AB);

    F.CStore(5000, Ord('H'));
    F.CStore(5001, Ord('I'));
    F.CStore(5002, Ord('!'));
    F.PushDS(5100);
    F.PushDS(5000);
    F.PushDS(3);
    F.WordCMove;
    AssertTrue('CMOVE dest[0]', F.CFetch(5100) = Ord('H'));
    AssertTrue('CMOVE dest[2]', F.CFetch(5102) = Ord('!'));

    { Overlapping forward-unsafe region: CMOVE> copies high to low. }
    F.CStore(5200, 1);
    F.CStore(5201, 2);
    F.CStore(5202, 3);
    F.PushDS(5201);
    F.PushDS(5200);
    F.PushDS(2);
    F.WordCMoveGreater;
    AssertTrue('CMOVE> overlap [1]', F.CFetch(5201) = 1);
    AssertTrue('CMOVE> overlap [2]', F.CFetch(5202) = 2);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestCellAndByteAccessPrimitives;
var
  F: TForth;
  OutStream: TMemoryStream;
begin
  WriteLn('--- TestCellAndByteAccessPrimitives ---');
  OutStream := TMemoryStream.Create;
  F := TForth.Create(nil, OutStream);
  try
    F.PushDS($1234);
    F.PushDS(1024);
    F.WordStore;
    F.PushDS(1024);
    F.WordFetch;
    AssertTrue('@ after ! yields 0x1234', F.PopDS = $1234);
    AssertTrue('@ leaves DS empty', F.DepthDS = 0);

    F.PushDS(Ord('A'));
    F.PushDS(3000);
    F.WordCStore;
    F.PushDS(3000);
    F.WordCFetch;
    AssertTrue('C@ after C! yields A', F.PopDS = Ord('A'));

    F.StoreCell(2200, 120);
    F.PushDS(5);
    F.PushDS(2200);
    F.WordPlusStore;
    AssertTrue('+! adds 5', F.FetchCell(2200) = 125);
    F.PushDS(-50);
    F.PushDS(2200);
    F.WordPlusStore;
    AssertTrue('+! adds negative', F.FetchCell(2200) = 75);
  finally
    F.Free;
    OutStream.Free;
  end;
end;

procedure TestQuitAbortCold;
var
  F: TForth;
  InStream, OutStream: TMemoryStream;
  OutText: String;
  Raised: Boolean;
begin
  WriteLn('--- TestQuitAbortCold ---');

  InStream := TMemoryStream.Create;
  OutStream := TMemoryStream.Create;
  F := TForth.Create(InStream, OutStream);
  try
    AssertTrue('cold start loads CORE', Length(F.LoadedModules) >= 1);
    AssertTrue('first loaded module is CORE', F.LoadedModules[0] = 'CORE');
    F.Quit;
    SetLength(OutText, OutStream.Size);
    if OutStream.Size > 0 then
    begin
      OutStream.Position := 0;
      OutStream.Read(OutText[1], OutStream.Size);
    end;
    AssertTrue('QUIT prints ok prompt then stops at EOF', OutText = ' ok' + #10);
  finally
    F.Free;
    InStream.Free;
    OutStream.Free;
  end;

  InStream := TMemoryStream.Create;
  OutStream := TMemoryStream.Create;
  F := TForth.Create(InStream, OutStream);
  try
    F.PushDS(1);
    F.PushRS(2);
    F.Interrupted := True;
    Raised := False;
    try
      F.CheckInterrupt;
    except
      on E: EForthError do
      begin
        Raised := True;
        AssertTrue('interrupt message', E.Message = 'INTERRUPTED');
      end;
    end;
    AssertTrue('CheckInterrupt raises', Raised);
    AssertTrue('interrupt clears data stack', F.DepthDS = 0);
    AssertTrue('interrupt clears return stack', F.DepthRS = 0);
  finally
    F.Free;
    InStream.Free;
    OutStream.Free;
  end;
end;

begin
  WriteLn('=== Running Free Pascal Test Runner ===');
  TestCLIArgDispatch;
  TestConsoleIOPrimitives;
  TestLineReaderAndTIB;
  TestQuitAbortCold;
  TestMemoryModelAndEndianness;
  TestCellAndByteAccessPrimitives;
  TestMemoryBlockOperations;
  TestBasicStackWords;
  TestShuffleStackWords;
  TestDoubleStackWords;
  TestReturnStackWords;
  TestSignedArithmetic;
  TestScalingHelpers;
  TestBitwiseLogic;
  TestComparisonFlags;
  TestDictionaryFindTickBody;
  TestDictionaryPointerComma;
  TestExecuteInnerInterpreter;
  TestDefiningWords;
  TestWordNumberBase;
  TestInterpretAndLit;
  TestCompilerStateWords;
  TestColonDefinitions;
  TestBranchRuntime;
  TestIfElseThen;
  TestBeginLoops;
  TestDoLoop;
  TestNumericOutput;
  TestStringsAndComments;
  TestEvaluateAndInclude;
  TestRequireAndCore;

  WriteLn('----------------------------------------');
  WriteLn(Format('Tests: %d | Passed: %d | Failed: %d', [TotalTests, PassedTests, FailedTests]));
  if FailedTests > 0 then
    Halt(1)
  else
    Halt(0);
end.
