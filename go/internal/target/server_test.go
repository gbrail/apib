package target

import (
	"bytes"
	"fmt"
	"io"
	"math/rand"
	"net/http"
	"net/http/httptest"
	"reflect"
	"strconv"
	"strings"
	"testing"
	"testing/quick"
)

var testRand = rand.New(rand.NewSource(0))

func TestPlain(t *testing.T) {
	svr := httptest.NewServer(makeHandler())
	defer svr.Close()

	t.Run("plain", func(t *testing.T) {
		serverTests(t, svr)
	})
}

func TestTLS(t *testing.T) {
	svr := httptest.NewTLSServer(makeHandler())
	defer svr.Close()

	t.Run("tls", func(t *testing.T) {
		serverTests(t, svr)
	})
}

func serverTests(t *testing.T, svr *httptest.Server) {
	t.Run("root", func(t *testing.T) {
		testRoot(t, svr)
	})
	t.Run("hello", func(t *testing.T) {
		testHello(t, svr)
	})
	t.Run("echo 100", func(t *testing.T) {
		testEcho(t, svr, 100, false)
	})
	t.Run("echo 100,000", func(t *testing.T) {
		testEcho(t, svr, 100000, false)
	})
	t.Run("echo 100 content-length", func(t *testing.T) {
		testEcho(t, svr, 100, true)
	})
	t.Run("echo 100,000 content-length", func(t *testing.T) {
		testEcho(t, svr, 100000, true)
	})
	t.Run("data 100", func(t *testing.T) {
		testData(t, svr, 100)
	})
	t.Run("data 100,000", func(t *testing.T) {
		testData(t, svr, 100000)
	})
}

func testRoot(t *testing.T, svr *httptest.Server) {
	resp, err := svr.Client().Get(svr.URL)
	if err != nil {
		t.Fatalf("Error on GET: %v", err)
	}
	if resp.StatusCode != 200 {
		t.Errorf("Invalid HTTP status. want 200, got %d", resp.StatusCode)
	}
}

func testHello(t *testing.T, svr *httptest.Server) {
	resp, err := svr.Client().Get(fmt.Sprintf("%s/hello", svr.URL))
	if err != nil {
		t.Fatalf("Error on GET: %v", err)
	}
	if resp.StatusCode != 200 {
		t.Errorf("Invalid HTTP status. want 200, got %d", resp.StatusCode)
	}
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("Error on read: %v", err)
	}
	if !strings.Contains(string(body), "Hello") {
		t.Errorf("Expected %q to contain \"Hello\"", string(body))
	}
}

func testEcho(t *testing.T, svr *httptest.Server, size int, contentLength bool) {
	requestBytes := randomBytes(size)
	req, err := http.NewRequest("POST", fmt.Sprintf("%s/echo", svr.URL), bytes.NewBuffer(requestBytes))
	if err != nil {
		t.Fatalf("Error making request: %v", err)
	}
	expectedContentLength := ""
	if contentLength {
		expectedContentLength = strconv.Itoa(size)
		req.Header.Set("Content-Length", expectedContentLength)
	}
	resp, err := svr.Client().Do(req)
	if err != nil {
		t.Fatalf("Error on POST: %v", err)
	}
	if resp.StatusCode != 200 {
		t.Errorf("Invalid HTTP status. want 200, got %d", resp.StatusCode)
	}
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("Error on read: %v", err)
	}
	if !bytes.Equal(body, requestBytes) {
		t.Errorf("Expected %d bytes to be the same, got %d", size, len(body))
	}
	gotContentLength := resp.Header.Get("Content-Length")
	if expectedContentLength != "" && gotContentLength != expectedContentLength {
		t.Errorf("Invalid content length. want = %q, got %q", expectedContentLength, gotContentLength)
	}
}

func testData(t *testing.T, svr *httptest.Server, size int) {
	resp, err := svr.Client().Get(fmt.Sprintf("%s/data?size=%d", svr.URL, size))
	if err != nil {
		t.Fatalf("Error on GET: %v", err)
	}
	if resp.StatusCode != 200 {
		t.Errorf("Invalid HTTP status. want 200, got %d", resp.StatusCode)
	}
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		t.Fatalf("Error on read: %v", err)
	}
	if len(body) != size {
		t.Errorf("Wrong response size. want = %d, got %d", size, len(body))
	}
}

func randomBytes(len int) []byte {
	val, _ := quick.Value(reflect.ArrayOf(len, reflect.TypeOf(byte(0))), testRand)
	return val.Bytes()
}
