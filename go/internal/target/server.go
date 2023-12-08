package target

import (
	"crypto/tls"
	"fmt"
	"io"
	"net"
	"net/http"
	"strconv"

	"github.com/julienschmidt/httprouter"
)

const (
	defaultContentType = "application/octet-stream"
	tmpBufLen          = 8192
)

type Server struct {
	port int
	tls  bool
}

func NewServer() *Server {
	return &Server{}
}

func (s *Server) SetPort(port int) {
	s.port = port
}

func (s *Server) SetTLS(tls bool) {
	s.tls = tls
}

func (s *Server) Run() error {
	var err error
	var listener net.Listener

	if s.tls {
		cert, key, err := MakeCertificate(7)
		if err != nil {
			return err
		}
		tlsCert, err := tls.X509KeyPair(cert, key)
		if err != nil {
			return fmt.Errorf("cannot create TLS certificate: %w", err)
		}
		tlsConf := &tls.Config{
			Certificates: []tls.Certificate{tlsCert},
		}
		listener, err = tls.Listen("tcp", fmt.Sprintf(":%d", s.port), tlsConf)
		if err != nil {
			return fmt.Errorf("failed to listen on port %d: %w", s.port, err)
		}

	} else {
		listener, err = net.Listen("tcp", fmt.Sprintf(":%d", s.port))
		if err != nil {
			return fmt.Errorf("failed to listen on port %d: %w", s.port, err)
		}
	}
	return http.Serve(listener, makeHandler())
}

func makeHandler() http.Handler {
	router := httprouter.New()
	router.GET("/", doRoot)
	router.GET("/help", doHelp)
	router.GET("/hello", doHello)
	router.GET("/data", doData)
	router.POST("/echo", doEcho)
	return router
}

func doHelp(resp http.ResponseWriter, req *http.Request, _ httprouter.Params) {
	resp.Header().Add("content-type", "text/plain")
	resp.Write([]byte(
		`Supported URLs:
/hello: Say hello
/echo: Echo back the request body
/data?size=xxx: Return "xxx" bytes of random data
/help: Print this message
`))
}

func doHello(resp http.ResponseWriter, req *http.Request, _ httprouter.Params) {
	resp.Header().Add("content-type", "text/plain")
	resp.Write([]byte("Hello, World!\n"))
}

func doRoot(resp http.ResponseWriter, req *http.Request, _ httprouter.Params) {
	resp.Header().Add("content-type", "text/plain")
	resp.Write([]byte("Hello! Use /help for options\n"))
}

func doEcho(resp http.ResponseWriter, req *http.Request, _ httprouter.Params) {
	cType := req.Header.Get("Content-Type")
	if cType == "" {
		cType = defaultContentType
	}
	cLength := req.Header.Get("Content-Length")

	// Go won't let us stream -- we need to read the whole request body
	// and then write back the response body. This (correctly) fails on tests!
	requestBody, err := io.ReadAll(req.Body)
	if err != nil {
		resp.WriteHeader(500)
		return
	}

	resp.Header().Set("Content-Type", cType)
	if cLength != "" {
		resp.Header().Set("Content-Length", cLength)
	}
	resp.Write(requestBody)
}

func doData(resp http.ResponseWriter, req *http.Request, _ httprouter.Params) {
	size := 0
	sizeStr := req.URL.Query().Get("size")
	if sizeStr != "" {
		size, _ = strconv.Atoi(sizeStr)
	}
	tmp := make([]byte, tmpBufLen)
	for i := range tmp {
		tmp[i] = 'a'
	}
	for size > 0 {
		writeSize := size
		if writeSize > tmpBufLen {
			writeSize = tmpBufLen
		}
		_, err := resp.Write(tmp[:writeSize])
		if err != nil {
			break
		}
		size -= writeSize
	}
}
