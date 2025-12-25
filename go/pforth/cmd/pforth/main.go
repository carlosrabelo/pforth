package main

import (
	"fmt"
	"io"
	"os"

	"github.com/carlosrabelo/pforth/pforth/internal/forth"
)

type Config struct {
	FilePaths   []string
	Interactive bool
}

func ParseArgs(args []string) Config {
	cfg := Config{}
	if len(args) > 1 {
		cfg.FilePaths = append(cfg.FilePaths, args[1:]...)
	}
	cfg.Interactive = (len(args) <= 1)
	return cfg
}

func Run(cfg Config, in io.Reader, out io.Writer, errOut io.Writer) int {
	f := forth.New(in, out)
	f.InitPrimitives()
	f.RequireModule("CORE")
	for _, path := range cfg.FilePaths {
		if err := f.LoadFile(path); err != nil {
			fmt.Fprintln(errOut, err)
			return 1
		}
	}
	if cfg.Interactive || len(cfg.FilePaths) == 0 {
		f.QUIT()
	}
	return 0
}

func main() {
	cfg := ParseArgs(os.Args)
	code := Run(cfg, os.Stdin, os.Stdout, os.Stderr)
	if code != 0 {
		os.Exit(code)
	}
}
