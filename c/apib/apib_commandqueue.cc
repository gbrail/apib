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

#include "apib/apib_commandqueue.h"

namespace apib {

void CommandQueue::Add(Command cmd) {
  std::lock_guard<std::mutex> l(lock_);
  commands_.push_back(cmd);
}

bool CommandQueue::Pop(Command* dest) {
  std::lock_guard<std::mutex> l(lock_);
  if (commands_.empty()) {
    return false;
  }

  *dest = commands_.front();
  commands_.pop_front();
  return true;
}

}  // namespace apib
