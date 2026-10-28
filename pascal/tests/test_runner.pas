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

begin
  WriteLn('=== Running Free Pascal Test Runner ===');
  TestCLIArgDispatch;
  TestConsoleIOPrimitives;

  WriteLn('----------------------------------------');
  WriteLn(Format('Tests: %d | Passed: %d | Failed: %d', [TotalTests, PassedTests, FailedTests]));
  if FailedTests > 0 then
    Halt(1)
  else
    Halt(0);
end.
