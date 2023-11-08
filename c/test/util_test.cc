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

#include "apib/apib_util.h"
#include "gtest/gtest.h"

using apib::eqcase;

namespace {

TEST(Eqcase, Success) {
  EXPECT_EQ(true, eqcase("", ""));
  EXPECT_EQ(true, eqcase("Hello", "Hello"));
  EXPECT_EQ(true, eqcase("Hello", "hello"));
  EXPECT_EQ(true, eqcase("hello, world!", "HELLO, World!"));
}

TEST(Eqcase, Failure) {
  EXPECT_EQ(false, eqcase("Hello", "Hello, World!"));
  EXPECT_EQ(false, eqcase("Hello", "ByeNo"));
  EXPECT_EQ(false, eqcase(" ", ""));
}

}  // namespace
