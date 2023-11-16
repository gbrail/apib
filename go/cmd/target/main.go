package main

import (
	"flag"
	"fmt"
	"os"

	"github.com/gbrail/apib/internal/target"
)

func main() {
	var port int
	var doHelp bool

	flag.IntVar(&port, "p", 0, "Listen port")
	flag.BoolVar(&doHelp, "h", false, "Print this message")
	flag.Parse()
	if !flag.Parsed() || doHelp {
		flag.PrintDefaults()
		os.Exit(2)
	}

	svr := target.NewServer()
	svr.SetPort(port)
	err := svr.Run()
	if err != nil {
		fmt.Printf("Error: %q\n", err)
		os.Exit(3)
	}
}
