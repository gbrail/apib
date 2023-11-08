package main

import (
	"context"
	"flag"
	"fmt"
	"os"
	"time"

	"github.com/gbrail/apib/apib"
)

const defaultTimeout = 60 * time.Second

func main() {
	var doHelp bool
	var verbose bool
	var method string

	flag.BoolVar(&doHelp, "h", false, "Print this message")
	flag.BoolVar(&verbose, "v", false, "Print verbose output")
	flag.StringVar(&method, "x", "GET", "HTTP method")
	flag.Parse()
	if !flag.Parsed() || doHelp || flag.NArg() != 1 {
		flag.PrintDefaults()
		os.Exit(1)
	}
	url := flag.Args()[0]

	sender, err := apib.NewSender(url)
	if err != nil {
		fmt.Printf("Error: %v\n", err)
		os.Exit(2)
	}
	sender.SetMethod(method)
	sender.SetVerbose(verbose)

	ctx, cancel := context.WithTimeout(context.Background(), defaultTimeout)
	defer cancel()
	err = sender.Send(ctx)
	if err != nil {
		fmt.Printf("Error: %v\n", err)
		os.Exit(3)
	}
}
