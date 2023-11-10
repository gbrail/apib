package apib

import (
	"context"
	"fmt"
	"math/rand"
	"net/http"
	"net/http/httptest"
	"reflect"
	"testing"
	"testing/quick"
)

var testRand = rand.New(rand.NewSource(0))

func TestSender(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(httpHandler))
	defer server.Close()

	t.Run("Get 100", func(t *testing.T) {
		testGet100(t, server.Client(), server.URL)
	})
	t.Run("Get 404", func(t *testing.T) {
		testGet404(t, server.Client(), server.URL)
	})
}

func testGet100(t *testing.T, client *http.Client, url string) {
	sender, err := NewSender(fmt.Sprintf("%s/get100", url), 1)
	if err != nil {
		t.Fatalf("Error creating sender: %v", err)
	}
	err = sender.Send(context.Background())
	if err != nil {
		t.Fatalf("Error on send: %v", err)
	}
}

func testGet404(t *testing.T, client *http.Client, url string) {
	sender, err := NewSender(fmt.Sprintf("%s/notfound", url), 1)
	if err != nil {
		t.Fatalf("Error creating sender: %v", err)
	}
	err = sender.Send(context.Background())
	if err == nil {
		t.Fatal("Expected 404")
	}
}

func httpHandler(resp http.ResponseWriter, req *http.Request) {
	if req.URL.Path == "/get100" {
		if req.Method == "GET" {
			resp.Write(randomBytes(100))
		} else {
			returnError(resp, 405)
		}
	} else {
		returnError(resp, 404)
	}
}

func returnError(resp http.ResponseWriter, status int) {
	resp.WriteHeader(status)
}

func randomBytes(len int) []byte {
	val, _ := quick.Value(reflect.ArrayOf(len, reflect.TypeOf(byte(0))), testRand)
	return val.Bytes()
}
