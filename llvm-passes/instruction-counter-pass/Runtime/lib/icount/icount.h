//===-- icount.h ------------------------------------------------*- C++ -*-===//
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
// Private ICount header.
//===----------------------------------------------------------------------===//

#ifndef DFSAN_H
#define DFSAN_H

#include <stdlib.h>
#include "sanitizer_common/sanitizer_internal_defs.h"
#include "icount_platform.h"

using __sanitizer::uptr;
using __sanitizer::u16;

// Copy declarations from public sanitizer/icount_interface.h header here.
typedef u16 icount_shadow;

extern "C" {
icount_shadow icount_transfer_shadow(icount_shadow shadow);
void icount_transfer_range_shadow(void *addr, uptr size);
icount_shadow icount_combine_shadows(icount_shadow s1, icount_shadow s2);
icount_shadow icount_get_range_shadow(const void *addr, size_t size);
icount_shadow icount_get_shadow(long data);
void icount_set_range_shadow(icount_shadow shadow, void *addr, uptr size);
void icount_enable_range_shadow(void *addr, uptr size);
void icount_disable_range_shadow(void *addr, uptr size);
}  // extern "C"

template <typename T>
void icount_enable_shadow(T &data) {  // NOLINT
  icount_enable_range_shadow(static_cast<void *>(&data), sizeof(T));
}

template <typename T>
void icount_disable_shadow(T &data) {  // NOLINT
  icount_disable_range_shadow(static_cast<void *>(&data), sizeof(T));
}

namespace __icount {
using namespace __sanitizer;

void InitializeInterceptors();

inline icount_shadow *shadow_for(void *ptr) {
  return (icount_shadow *) ((((uptr) ptr) & ShadowMask()) << 1);
}

inline const icount_shadow *shadow_for(const void *ptr) {
  return shadow_for(const_cast<void *>(ptr));
}

struct Flags {
#define ICOUNT_FLAG(Type, Name, DefaultValue, Description) Type Name;
#include "icount_flags.inc"
#undef ICOUNT_FLAG

  void SetDefaults();
};

extern Flags flags_data;
inline Flags &flags() {
  return flags_data;
}

}  // namespace __icount

#endif  // DFSAN_H
