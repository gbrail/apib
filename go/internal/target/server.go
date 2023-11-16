package target

import (
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
}

func NewServer() *Server {
	return &Server{}
}

func (s *Server) SetPort(port int) {
	s.port = port
}

func (s *Server) Run() error {
	router := httprouter.New()
	router.GET("/", doRoot)
	router.GET("/help", doHelp)
	router.GET("/hello", doHello)
	router.GET("/data", doData)
	router.POST("/echo", doEcho)

	listener, err := net.Listen("tcp", fmt.Sprintf(":%d", s.port))
	if err != nil {
		return fmt.Errorf("failed to listen on port %d: %w", s.port, err)
	}
	return http.Serve(listener, router)
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
	resp.Header().Set("Content-Type", cType)
	io.Copy(resp, req.Body)
}

func doData(resp http.ResponseWriter, req *http.Request, _ httprouter.Params) {
	size := 0
	sizeStr := req.URL.Query().Get("size")
	if sizeStr != "" {
		size, _ = strconv.Atoi(sizeStr)
	}
	tmp := make([]byte, tmpBufLen)
	for i, _ := range tmp {
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
