/*
Copyright 2019 Google LLC

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

#include <cstdlib>
#include <iostream>
#include <string>

#include "test/test_server.h"

int main(int argc, char** argv) {
  if ((argc < 2) || (argc > 4)) {
    std::cerr << "Usage: testserver <port> [<key file> <cert file>]"
              << std::endl;
    return 1;
  }

  int port = atoi(argv[1]);
  std::string keyFile;
  std::string certFile;

  if (argc > 2) {
    keyFile = argv[2];
  }
  if (argc > 3) {
    certFile = argv[3];
  }

  apib::TestServer svr;
  int err = svr.start("0.0.0.0", port, keyFile, certFile);
  if (err != 0) {
    return 2;
  }

  std::cout << "Listening on port " << svr.port() << std::endl;

  svr.join();
  return 0;
}