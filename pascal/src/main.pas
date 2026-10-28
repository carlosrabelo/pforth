program pforth;

{$mode objfpc}{$H+}

uses
  SysUtils, pforth_cli;

var
  Args: TStringArray;
  I: Integer;
  Config: TCLIConfig;
  ExitCodeVal: Integer;
begin
  SetLength(Args, ParamCount);
  for I := 1 to ParamCount do
    Args[I - 1] := ParamStr(I);

  Config := ParseArgs(Args);
  ExitCodeVal := RunCLI(Config);
  Halt(ExitCodeVal);
end.
