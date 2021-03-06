//===-- icount_interceptors.cc --------------------------------------------===//
//
//                     The LLVM Compiler Infrastructure
//
// This file is distributed under the University of Illinois Open Source
// License. See LICENSE.TXT for details.
//
//===----------------------------------------------------------------------===//
//
// This file is a part of InstructionCounter.
//
// Interceptors for standard library functions.
//===----------------------------------------------------------------------===//

#include "icount/icount.h"
#include "interception/interception.h"
#include "sanitizer_common/sanitizer_common.h"

using namespace __sanitizer;

INTERCEPTOR(void *, mmap, void *addr, SIZE_T length, int prot, int flags,
            int fd, OFF_T offset) {
  void *res = REAL(mmap)(addr, length, prot, flags, fd, offset);
  if (res != (void*)-1)
    icount_disable_range_shadow(res, RoundUpTo(length, GetPageSize()));
  return res;
}

INTERCEPTOR(void *, mmap64, void *addr, SIZE_T length, int prot, int flags,
            int fd, OFF64_T offset) {
  void *res = REAL(mmap64)(addr, length, prot, flags, fd, offset);
  if (res != (void*)-1)
    icount_disable_range_shadow(res, RoundUpTo(length, GetPageSize()));
  return res;
}

namespace __icount {
void InitializeInterceptors() {
  static int inited = 0;
  CHECK_EQ(inited, 0);

  INTERCEPT_FUNCTION(mmap);
  INTERCEPT_FUNCTION(mmap64);
  inited = 1;
}
}  // namespace __icount
