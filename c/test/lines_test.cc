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

#include "apib/apib_lines.h"
#include "gtest/gtest.h"

using apib::LineState;

namespace {

TEST(Lines, AllFull) {
  const char* const DATA = "Line one\nLine two\nLine three\n";
  auto realData = strdup(DATA);
  auto realLen = strlen(DATA);

  LineState l(realData, realLen, realLen);
  EXPECT_NE(false, l.next());
  EXPECT_EQ("Line one", l.line());
  EXPECT_NE(false, l.next());
  EXPECT_EQ("Line two", l.line());
  EXPECT_NE(false, l.next());
  EXPECT_EQ("Line three", l.line());
  EXPECT_EQ(false, l.next());
}

TEST(Lines, SlowFill) {
  // Empty buffer, no line
  const int bufLen = 100;
  LineState l(bufLen);

  EXPECT_EQ(false, l.next());
  EXPECT_EQ(true, l.line().empty());
  EXPECT_EQ(true, l.consume());

  // Add a line and a half
  const char* const CHUNK1 = "Line one\nLin";
  char* chunk = strdup(CHUNK1);
  auto chunkLen = strlen(CHUNK1);

  char* writePos;
  int spaceLeft;
  l.getReadInfo(&writePos, &spaceLeft);
  EXPECT_LE(chunkLen, spaceLeft);
  memcpy(writePos, chunk, chunkLen);
  l.setReadLength(chunkLen);
  free(chunk);

  // Now we can read the first line
  EXPECT_NE(false, l.next());
  EXPECT_EQ("Line one", l.line());
  EXPECT_EQ(false, l.next());

  // And now we can add the rest
  const char* const CHUNK2 = "e two\r\n\r\nLast line\n";
  chunk = strdup(CHUNK2);
  chunkLen = strlen(CHUNK2);
  EXPECT_EQ(true, l.consume());
  l.getReadInfo(&writePos, &spaceLeft);
  EXPECT_LE(chunkLen, spaceLeft);
  memcpy(writePos, chunk, chunkLen);
  l.setReadLength(chunkLen);
  free(chunk);

  // Now we should have two more lines
  EXPECT_NE(false, l.next());
  EXPECT_EQ("Line two", l.line());
  EXPECT_NE(false, l.next());
  EXPECT_EQ("Last line", l.line());
  EXPECT_EQ(false, l.next());
}

TEST(Lines, Tokens) {
  // Empty buffer, no line
  const int bufLen = 100;
  LineState l(bufLen);

  EXPECT_EQ(false, l.next());
  EXPECT_EQ(true, l.line().empty());
  EXPECT_EQ(true, l.consume());

  // Add half a line
  const char* const CHUNK1 = "Newval";
  char* chunk = strdup(CHUNK1);
  auto chunkLen = strlen(CHUNK1);

  char* writePos;
  int spaceLeft;
  l.getReadInfo(&writePos, &spaceLeft);
  EXPECT_LE(chunkLen, spaceLeft);
  memcpy(writePos, chunk, chunkLen);
  l.setReadLength(chunkLen);
  free(chunk);

  // No line. Now we need to add the rest
  EXPECT_EQ(false, l.next());
  EXPECT_EQ(true, l.consume());
  ;

  const char* const CHUNK2 = "ue: Foobar\n";
  chunk = strdup(CHUNK2);
  chunkLen = strlen(CHUNK2);
  l.getReadInfo(&writePos, &spaceLeft);
  EXPECT_LE(chunkLen, spaceLeft);
  memcpy(writePos, chunk, chunkLen);
  l.setReadLength(chunkLen);
  free(chunk);

  // Now we have a line with a token in it
  EXPECT_EQ(true, l.next());
  EXPECT_EQ("Newvalue", l.nextToken(": "));
  EXPECT_EQ("Foobar", l.nextToken(": "));
  EXPECT_EQ("", l.nextToken(": "));
}

TEST(Lines, HttpMode) {
  char* buf = strdup("One\r\nTwo\r\n\r\nThree\r\n\r\n");
  size_t len = strlen(buf);
  LineState l(buf, len + 1, len);
  l.setHttpMode(true);

  EXPECT_EQ(true, l.next());
  EXPECT_EQ("One", l.line());
  EXPECT_EQ(true, l.next());
  EXPECT_EQ("Two", l.line());
  EXPECT_EQ(true, l.next());
  EXPECT_EQ("", l.line());
  EXPECT_EQ(true, l.next());
  EXPECT_EQ("Three", l.line());
  EXPECT_EQ(true, l.next());
  EXPECT_EQ("", l.line());
  EXPECT_EQ(false, l.next());
}

TEST(Lines, TooLong) {
  // Empty buffer, no line
  const int bufLen = 20;
  LineState l(bufLen);

  EXPECT_EQ(false, l.next());
  EXPECT_EQ(true, l.line().empty());
  EXPECT_EQ(true, l.consume());

  // Add half a line
  const char* const CHUNK1 = "0123456789";
  char* chunk = strdup(CHUNK1);
  auto chunkLen = strlen(CHUNK1);

  char* writePos;
  int spaceLeft;
  l.getReadInfo(&writePos, &spaceLeft);
  EXPECT_LE(chunkLen, spaceLeft);
  memcpy(writePos, chunk, chunkLen);
  l.setReadLength(chunkLen);
  free(chunk);

  // No line. Now we need to add the rest
  EXPECT_EQ(false, l.next());
  EXPECT_EQ(true, l.consume());

  const char* const CHUNK2 = "0123456789";
  chunk = strdup(CHUNK2);
  chunkLen = strlen(CHUNK2);
  l.getReadInfo(&writePos, &spaceLeft);
  EXPECT_LE(chunkLen, spaceLeft);
  memcpy(writePos, chunk, chunkLen);
  l.setReadLength(chunkLen);
  free(chunk);

  // We still don't have a line, and we can't add one
  EXPECT_EQ(false, l.next());
  EXPECT_EQ(false, l.consume());
}

}  // namespace
