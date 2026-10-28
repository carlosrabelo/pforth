unit pforth_cli;

{$mode objfpc}{$H+}

interface

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
begin
  // Bootstrap placeholder
  Result := 0;
end;

end.
