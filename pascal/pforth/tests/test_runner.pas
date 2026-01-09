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

  AssertTrue('RunCLI returns 0', RunCLI(Cfg1) = 0);
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

  WriteLn('----------------------------------------');
  WriteLn(Format('Tests: %d | Passed: %d | Failed: %d', [TotalTests, PassedTests, FailedTests]));
  if FailedTests > 0 then
    Halt(1)
  else
    Halt(0);
end.
