package main

import (
	"flag"
	"fmt"
	"os"

	"github.com/gbrail/apib/internal/target"
)

func main() {
	var doHelp bool
	var certFile string
	var keyFile string
	var days int

	flag.BoolVar(&doHelp, "h", false, "Print this message")
	flag.StringVar(&certFile, "c", "", "Certificate file")
	flag.StringVar(&keyFile, "k", "", "Key file")
	flag.IntVar(&days, "d", 7, "Days for certificate to be valid")
	flag.Parse()
	if !flag.Parsed() || doHelp || certFile == "" || keyFile == "" {
		flag.PrintDefaults()
		os.Exit(1)
	}

	// TODO days
	certPem, keyPem, err := target.MakeCertificate(days)
	if err != nil {
		fmt.Printf("Error making certificate: %q\n", err)
	}

	err = os.WriteFile(certFile, certPem, 0600)
	if err != nil {
		fmt.Printf("Error writing cert file %q: %v\n", certFile, err)
	}
	err = os.WriteFile(keyFile, keyPem, 0600)
	if err != nil {
		fmt.Printf("Error writing key file %q: %v\n", keyFile, err)
	}
}
