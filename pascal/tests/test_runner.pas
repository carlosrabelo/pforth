program test_runner;

{$mode objfpc}{$H+}

uses
  SysUtils, pforth_cli;

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

begin
  WriteLn('=== Running Free Pascal Test Runner ===');
  TestCLIArgDispatch;

  WriteLn('----------------------------------------');
  WriteLn(Format('Tests: %d | Passed: %d | Failed: %d', [TotalTests, PassedTests, FailedTests]));
  if FailedTests > 0 then
    Halt(1)
  else
    Halt(0);
end.
