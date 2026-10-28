package main

import (
	"os"

	"github.com/carlosrabelo/pforth/pforth/internal/forth"
)

func main() {
	f := forth.New(os.Stdin, os.Stdout)
	_ = f
}
