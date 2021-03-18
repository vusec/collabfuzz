//===-- icount.cc ---------------------------------------------------------===//
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
// InstructionCounter runtime.  This file defines the public interface to
// InstructionCounter as well as the definition of certain runtime functions
// called automatically by the compiler (specifically the instrumentation pass
// in llvm/lib/Transforms/Instrumentation/DataFlowSanitizer.cpp).
//
// The public interface is defined in include/sanitizer/icount_interface.h whose
// functions are prefixed icount_ while the compiler interface functions are
// prefixed __icount_.
//===----------------------------------------------------------------------===//

#include "sanitizer_common/sanitizer_atomic.h"
#include "sanitizer_common/sanitizer_common.h"
#include "sanitizer_common/sanitizer_file.h"
#include "sanitizer_common/sanitizer_flags.h"
#include "sanitizer_common/sanitizer_flag_parser.h"
#include "sanitizer_common/sanitizer_libc.h"

#include "icount/icount.h"

using namespace __icount;

Flags __icount::flags_data;

SANITIZER_INTERFACE_ATTRIBUTE THREADLOCAL icount_shadow __icount_retval_tls;
SANITIZER_INTERFACE_ATTRIBUTE THREADLOCAL icount_shadow __icount_arg_tls[64];

SANITIZER_INTERFACE_ATTRIBUTE uptr __icount_shadow_ptr_mask;

// On Linux/x86_64, memory is laid out as follows:
//
// +--------------------+ 0x800000000000 (top of memory)
// | application memory |
// +--------------------+ 0x700000008000 (kAppAddr)
// |                    |
// |       unused       |
// |                    |
// +--------------------+ 0x200000000000 (kUnionTableAddr)
// |   shadow memory    |
// +--------------------+ 0x000000010000 (kShadowAddr)
// | reserved by kernel |
// +--------------------+ 0x000000000000
//
// To derive a shadow memory address from an application memory address,
// bits 44-46 are cleared to bring the address into the range
// [0x000000008000,0x100000000000).  Then the address is shifted left by 1 to
// account for the double byte representation of shadow labels and move the
// address into the shadow memory range.  See the function shadow_for below.

// On Linux/MIPS64, memory is laid out as follows:
//
// +--------------------+ 0x10000000000 (top of memory)
// | application memory |
// +--------------------+ 0xF000008000 (kAppAddr)
// |                    |
// |       unused       |
// |                    |
// +--------------------+ 0x2000000000 (kUnionTableAddr)
// |   shadow memory    |
// +--------------------+ 0x0000010000 (kShadowAddr)
// | reserved by kernel |
// +--------------------+ 0x0000000000

// On Linux/AArch64 (39-bit VMA), memory is laid out as follow:
//
// +--------------------+ 0x8000000000 (top of memory)
// | application memory |
// +--------------------+ 0x7000008000 (kAppAddr)
// |                    |
// |       unused       |
// |                    |
// +--------------------+ 0x1000000000 (kUnionTableAddr)
// |   shadow memory    |
// +--------------------+ 0x0000010000 (kShadowAddr)
// | reserved by kernel |
// +--------------------+ 0x0000000000

// On Linux/AArch64 (42-bit VMA), memory is laid out as follow:
//
// +--------------------+ 0x40000000000 (top of memory)
// | application memory |
// +--------------------+ 0x3ff00008000 (kAppAddr)
// |                    |
// |       unused       |
// |                    |
// +--------------------+ 0x8000000000 (kUnionTableAddr)
// |   shadow memory    |
// +--------------------+ 0x0000010000 (kShadowAddr)
// | reserved by kernel |
// +--------------------+ 0x0000000000

// On Linux/AArch64 (48-bit VMA), memory is laid out as follow:
//
// +--------------------+ 0x1000000000000 (top of memory)
// | application memory |
// +--------------------+ 0xffff00008000 (kAppAddr)
// |       unused       |
// +--------------------+ 0xaaaab0000000 (top of PIE address)
// | application PIE    |
// +--------------------+ 0xaaaaa0000000 (top of PIE address)
// |                    |
// |       unused       |
// |                    |
// +--------------------+ 0x8000000000 (kUnionTableAddr)
// |   shadow memory    |
// +--------------------+ 0x0000010000 (kShadowAddr)
// | reserved by kernel |
// +--------------------+ 0x0000000000

#ifdef ICOUNT_RUNTIME_VMA
// Runtime detected VMA size.
int __icount::vmaSize;
#endif

extern "C" SANITIZER_INTERFACE_ATTRIBUTE
icount_shadow icount_transfer_shadow(icount_shadow shadow) {
  return (shadow > 0) ? shadow + 1 : 0;
}

extern "C" SANITIZER_INTERFACE_ATTRIBUTE
icount_shadow icount_combine_shadows(icount_shadow s1, icount_shadow s2) {
    return (s1 > s2) ? s1 : s2;
}

extern "C" SANITIZER_INTERFACE_ATTRIBUTE
icount_shadow __icount_combine_on_load(const icount_shadow *shadows, uptr n) {
  icount_shadow shadow = shadows[0];
  for (uptr i = 1; i != n; ++i) {
    icount_shadow next_shadow = shadows[i];
    shadow = icount_combine_shadows(next_shadow, shadow);
  }
  return shadow;
}

extern "C" SANITIZER_INTERFACE_ATTRIBUTE icount_shadow
icount_get_range_shadow(const void *addr, uptr size) {
  if (size == 0)
    return 0;
  return __icount_combine_on_load(shadow_for(addr), size);
}

// Unlike the other icount interface functions the behavior of this function
// depends on the shadow of one of its arguments.  Hence it is implemented as a
// custom function.
extern "C" SANITIZER_INTERFACE_ATTRIBUTE icount_shadow
__icountw_icount_get_shadow(long data, icount_shadow data_shadow,
                            icount_shadow *ret_shadow) {
  *ret_shadow = 0;
  return data_shadow;
}

extern "C" SANITIZER_INTERFACE_ATTRIBUTE
void icount_set_range_shadow(icount_shadow shadow, void *addr, uptr size) {
  for (icount_shadow *shadowp = shadow_for(addr); size != 0; --size, ++shadowp) {
    // Don't write the shadow if it is already the value we need it to be.
    // In a program where most addresses are not labeled, it is common that
    // a page of shadow memory is entirely zeroed.  The Linux copy-on-write
    // implementation will share all of the zeroed pages, making a copy of a
    // page when any value is written.  The un-sharing will happen even if
    // the value written does not change the value in memory.  Avoiding the
    // write when both |shadow| and |*shadowp| are zero dramatically reduces
    // the amount of real memory used by large programs.
    if (shadow == *shadowp)
      continue;

    *shadowp = shadow;
  }
}

extern "C" SANITIZER_INTERFACE_ATTRIBUTE
void icount_enable_range_shadow(void *addr, uptr size) {
  icount_set_range_shadow(1, addr, size);
}

extern "C" SANITIZER_INTERFACE_ATTRIBUTE
void icount_disable_range_shadow(void *addr, uptr size) {
  icount_set_range_shadow(0, addr, size);
}

extern "C" SANITIZER_INTERFACE_ATTRIBUTE
void __icount_unimplemented(char *fname) {
  if (flags().warn_unimplemented)
    Report("WARNING: InstructionCounter: call to uninstrumented function %s\n",
           fname);
}

// Use '-mllvm -icount-debug-nonzero-shadows' and break on this function
// to try to figure out where shadows are being introduced in a nominally
// shadow-free program.
extern "C" SANITIZER_INTERFACE_ATTRIBUTE void __icount_nonzero_shadow() {
  if (flags().warn_nonzero_shadows)
    Report("WARNING: InstructionCounter: saw nonzero shadow\n");
}

// Indirect call to an uninstrumented vararg function. We don't have a way of
// handling these at the moment.
extern "C" SANITIZER_INTERFACE_ATTRIBUTE void
__icount_vararg_wrapper(const char *fname) {
  Report("FATAL: InstructionCounter: unsupported indirect call to vararg "
         "function %s\n", fname);
  Die();
}

void Flags::SetDefaults() {
#define ICOUNT_FLAG(Type, Name, DefaultValue, Description) Name = DefaultValue;
#include "icount_flags.inc"
#undef ICOUNT_FLAG
}

static void RegisterDfsanFlags(FlagParser *parser, Flags *f) {
#define ICOUNT_FLAG(Type, Name, DefaultValue, Description) \
  RegisterFlag(parser, #Name, Description, &f->Name);
#include "icount_flags.inc"
#undef ICOUNT_FLAG
}

static void InitializeFlags() {
  SetCommonFlagsDefaults();
  flags().SetDefaults();

  FlagParser parser;
  RegisterCommonFlags(&parser);
  RegisterDfsanFlags(&parser, &flags());
  parser.ParseString(GetEnv("ICOUNT_OPTIONS"));
  InitializeCommonFlags();
  if (Verbosity()) ReportUnrecognizedFlags();
  if (common_flags()->help) parser.PrintFlagDescriptions();
}

static void InitializePlatformEarly() {
  AvoidCVE_2016_2143();
#ifdef ICOUNT_RUNTIME_VMA
  __icount::vmaSize =
    (MostSignificantSetBitIndex(GET_CURRENT_FRAME()) + 1);
  if (__icount::vmaSize == 39 || __icount::vmaSize == 42 ||
      __icount::vmaSize == 48) {
    __icount_shadow_ptr_mask = ShadowMask();
  } else {
    Printf("FATAL: InstructionCounter: unsupported VMA range\n");
    Printf("FATAL: Found %d - Supported 39, 42, and 48\n", __icount::vmaSize);
    Die();
  }
#endif
}

static void icount_init(int argc, char **argv, char **envp) {
  InitializeFlags();

  ::InitializePlatformEarly();

  if (!MmapFixedNoReserve(ShadowAddr(), UnionTableAddr() - ShadowAddr()))
    Die();

  // Protect the region of memory we don't use, to preserve the one-to-one
  // mapping from application to shadow memory. But if ASLR is disabled, Linux
  // will load our executable in the middle of our unused region. This mostly
  // works so long as the program doesn't use too much memory. We support this
  // case by disabling memory protection when ASLR is disabled.
  uptr init_addr = (uptr)&icount_init;
  if (!(init_addr >= UnionTableAddr() && init_addr < AppAddr()))
    MmapFixedNoAccess(UnionTableAddr(), AppAddr() - UnionTableAddr());

  InitializeInterceptors();
}

#if SANITIZER_CAN_USE_PREINIT_ARRAY
__attribute__((section(".preinit_array"), used))
static void (*icount_init_ptr)(int, char **, char **) = icount_init;
#endif
