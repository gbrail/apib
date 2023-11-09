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

const (
	defaultTimeout = 60 * time.Second
	printInterval  = 5 * time.Second
)

func main() {
	var doHelp bool
	var verbose bool
	var method string
	var durationSecs int
	var concurrency int
	var justOnce bool

	flag.BoolVar(&doHelp, "h", false, "Print this message")
	flag.BoolVar(&verbose, "v", false, "Print verbose output")
	flag.StringVar(&method, "x", "GET", "HTTP method")
	flag.BoolVar(&justOnce, "1", false, "Send just one request")
	flag.IntVar(&durationSecs, "d", 60, "Test run duration in seconds")
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
		duration := time.Duration(durationSecs) * time.Second
		collector := apib.NewCollector()
		wg := &sync.WaitGroup{}
		wg.Add(concurrency)
		time.AfterFunc(duration, collector.Stop)
		printTicker := time.NewTicker(printInterval)
		startTime := time.Now()
		tickStart := startTime
		go func() {
			for {
				<-printTicker.C
				tickStart = collector.WriteTick(startTime, tickStart, duration, os.Stdout)
			}
		}()
		for i := 0; i < concurrency; i++ {
			go func() {
				sender.Loop(collector)
				wg.Done()
			}()
		}
		wg.Wait()
		collector.Write(startTime, time.Now(), os.Stdout)
	}
}
