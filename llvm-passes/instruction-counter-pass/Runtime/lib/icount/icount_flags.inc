//===-- dfsan_flags.inc -----------------------------------------*- C++ -*-===//
//
//                     The LLVM Compiler Infrastructure
//
// This file is distributed under the University of Illinois Open Source
// License. See LICENSE.TXT for details.
//
//===----------------------------------------------------------------------===//
//
// ICount runtime flags.
//
//===----------------------------------------------------------------------===//
#ifndef ICOUNT_FLAG
# error "Define ICOUNT_FLAG prior to including this file!"
#endif

// ICOUNT_FLAG(Type, Name, DefaultValue, Description)
// See COMMON_FLAG in sanitizer_flags.inc for more details.

ICOUNT_FLAG(bool, warn_unimplemented, true,
           "Whether to warn on unimplemented functions.")
ICOUNT_FLAG(bool, warn_nonzero_shadows, false,
           "Whether to warn on unimplemented functions.")
