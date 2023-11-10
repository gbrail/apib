package apib

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"net/http"
	"net/http/httputil"
	"net/url"
	"os"
	"time"
)

const defaultTimeout = 60 * time.Second

type Sender struct {
	client  *http.Client
	url     *url.URL
	urlStr  string
	method  string
	verbose bool
}

func NewSender(urlStr string, expectedConnections int) (*Sender, error) {
	u, err := url.Parse(urlStr)
	if err != nil {
		return nil, fmt.Errorf("invalid URL: %q: %w", urlStr, err)
	}
	return &Sender{
		client: &http.Client{
			Transport: &http.Transport{
				MaxIdleConns:        expectedConnections * 2,
				MaxIdleConnsPerHost: expectedConnections * 2,
				MaxConnsPerHost:     expectedConnections * 2,
			},
		},
		url:    u,
		urlStr: urlStr,
		method: "GET",
	}, nil
}

func (s *Sender) SetVerbose(verbose bool) {
	s.verbose = verbose
}

func (s *Sender) SetMethod(method string) {
	s.method = method
}

func (s *Sender) Send(ctx context.Context) error {
	req, err := http.NewRequestWithContext(ctx, s.method, s.urlStr, &bytes.Buffer{})
	if err != nil {
		return fmt.Errorf("error creating request: %w", err)
	}
	if s.verbose {
		dump, err := httputil.DumpRequestOut(req, false)
		if err != nil {
			return fmt.Errorf("error on dump: %w", err)
		}
		os.Stdout.Write(dump)
	}
	resp, err := s.client.Do(req)
	if err != nil {
		ret := fmt.Errorf("request error: %w", err)
		if s.verbose {
			fmt.Printf("%v\n", ret)
		}
		return ret
	}
	defer resp.Body.Close()
	if s.verbose {
		dump, err := httputil.DumpResponse(resp, false)
		if err != nil {
			return fmt.Errorf("error on dump: %w", err)
		}
		os.Stdout.Write(dump)
	}
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("received error status: %d", resp.StatusCode)
	}
	tmp := make([]byte, 8192)
	bytesRead := 0
	for err == nil {
		var r int
		r, err = resp.Body.Read(tmp)
		bytesRead += r
	}
	if err != io.EOF {
		ret := fmt.Errorf("received error %w after reading %d bytes", err, bytesRead)
		if s.verbose {
			fmt.Printf("%v\n", ret)
		}
		return ret
	}
	if s.verbose {
		fmt.Printf("Response body: %d bytes\n", bytesRead)
	}
	return nil
}

func (s *Sender) Loop(ctx context.Context, c *Collector) {
	pleaseStop := false
	for !pleaseStop {
		reqCtx, cancel := context.WithTimeout(ctx, defaultTimeout)
		defer cancel()
		start := time.Now()
		err := s.Send(reqCtx)
		if err == nil {
			pleaseStop = c.Success(start, 0, 0)
		} else {
			pleaseStop = c.Failure(err)
		}
	}
}
