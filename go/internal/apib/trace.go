package apib

import (
	"fmt"
	"net/http/httptrace"
)

func MakeTracer() *httptrace.ClientTrace {
	return &httptrace.ClientTrace{
		GetConn: func(hostPort string) {
			fmt.Printf("Connecting to %q\n", hostPort)
		},
		GotConn: func(info httptrace.GotConnInfo) {
			fmt.Printf("  connected reused = %t wasIdle = %t\n", info.Reused, info.WasIdle)
		},
		PutIdleConn: func(err error) {
			fmt.Printf("Put idle conn: %v\n", err)
		},
		DNSStart: func(info httptrace.DNSStartInfo) {
			fmt.Printf("DNS start: %q\n", info.Host)
		},
		DNSDone: func(info httptrace.DNSDoneInfo) {
			fmt.Printf("DNS done: err = %v\n", info.Err)
		},
		ConnectStart: func(network, addr string) {
			fmt.Printf("Connect start: %s:%s\n", network, addr)
		},
		ConnectDone: func(network, addr string, err error) {
			fmt.Printf("  connected: err = %v\n", err)
		},
	}
}
