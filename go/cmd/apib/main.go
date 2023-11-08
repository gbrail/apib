package main

import (
	"context"
	"flag"
	"fmt"
	"os"
	"sync"
	"time"

	"github.com/gbrail/apib/apib"
)

const defaultTimeout = 60 * time.Second

func main() {
	var doHelp bool
	var verbose bool
	var method string
	var duration int
	var concurrency int
	var justOnce bool

	flag.BoolVar(&doHelp, "h", false, "Print this message")
	flag.BoolVar(&verbose, "v", false, "Print verbose output")
	flag.StringVar(&method, "x", "GET", "HTTP method")
	flag.BoolVar(&justOnce, "1", false, "Send just one request")
	flag.IntVar(&duration, "d", 60, "Test run duration in seconds")
	flag.IntVar(&concurrency, "c", 1, "Number of parallel requests")
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

	if justOnce {
		ctx, cancel := context.WithTimeout(context.Background(), defaultTimeout)
		defer cancel()
		err = sender.Send(ctx)
		if err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(3)
		}
	} else {
		collector := apib.NewCollector()
		wg := &sync.WaitGroup{}
		wg.Add(concurrency)
		time.AfterFunc(time.Duration(duration)*time.Second, collector.Stop)
		start := time.Now()
		for i := 0; i < concurrency; i++ {
			go func() {
				sender.Loop(collector)
				wg.Done()
			}()
		}
		wg.Wait()
		collector.Write(start, time.Now(), os.Stdout)
	}
}
