unit pforth_cli;

{$mode objfpc}{$H+}

interface

uses
  SysUtils, Classes, pforth_types, pforth_engine;

type
  TStringArray = array of String;

  TCLIConfig = record
    FilePaths: TStringArray;
    Interactive: Boolean;
  end;

function ParseArgs(const Args: TStringArray): TCLIConfig;
function RunCLI(const Config: TCLIConfig): Integer;

implementation

function ParseArgs(const Args: TStringArray): TCLIConfig;
var
  I, Count: Integer;
begin
  Count := Length(Args);
  if Count = 0 then
  begin
    Result.Interactive := True;
    SetLength(Result.FilePaths, 0);
  end
  else
  begin
    Result.Interactive := False;
    SetLength(Result.FilePaths, Count);
    for I := 0 to Count - 1 do
      Result.FilePaths[I] := Args[I];
  end;
end;

function RunCLI(const Config: TCLIConfig): Integer;
var
  F: TForth;
  I: Integer;
  Path: String;
begin
  F := TForth.Create(nil, nil);
  try
    for I := 0 to High(Config.FilePaths) do
    begin
      Path := Config.FilePaths[I];
      try
        F.LoadFile(Path);
      except
        on E: EForthError do
        begin
          WriteLn(E.Message);
          Result := 1;
          Exit;
        end;
      end;
    end;
    if Config.Interactive then
      F.Quit;
    Result := 0;
  finally
    F.Free;
  end;
end;

end.
