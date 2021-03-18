include_directories(..)

# Runtime library sources and build flags.
set(DFSAN_RTL_SOURCES
  icount.cc
  icount_custom.cc
  icount_interceptors.cc)

set(DFSAN_RTL_HEADERS
  icount.h
  icount_flags.inc
  icount_platform.h)

set(DFSAN_COMMON_CFLAGS ${SANITIZER_COMMON_CFLAGS})
append_rtti_flag(OFF DFSAN_COMMON_CFLAGS)
# Prevent clang from generating libc calls.
append_list_if(COMPILER_RT_HAS_FFREESTANDING_FLAG -ffreestanding DFSAN_COMMON_CFLAGS)

# Static runtime library.
add_compiler_rt_component(icount)

foreach(arch ${DFSAN_SUPPORTED_ARCH})
  set(DFSAN_CFLAGS ${DFSAN_COMMON_CFLAGS})
  append_list_if(COMPILER_RT_HAS_FPIE_FLAG -fPIE DFSAN_CFLAGS)
  add_compiler_rt_runtime(clang_rt.icount
    STATIC
    ARCHS ${arch}
    SOURCES ${DFSAN_RTL_SOURCES}
            $<TARGET_OBJECTS:RTInterception.${arch}>
            $<TARGET_OBJECTS:RTSanitizerCommon.${arch}>
            $<TARGET_OBJECTS:RTSanitizerCommonLibc.${arch}>
    ADDITIONAL_HEADERS ${DFSAN_RTL_HEADERS}
    CFLAGS ${DFSAN_CFLAGS}
    PARENT_TARGET icount)
  add_sanitizer_rt_symbols(clang_rt.icount
    ARCHS ${arch}
    EXTRA icount.syms.extra)
  add_dependencies(icount
    clang_rt.icount-${arch}-symbols)
endforeach()

set(icount_abilist_dir ${COMPILER_RT_OUTPUT_DIR}/share)
set(icount_abilist_filename ${icount_abilist_dir}/icount_abilist.txt)
add_custom_target(icount_abilist ALL
  DEPENDS ${icount_abilist_filename})
add_custom_command(OUTPUT ${icount_abilist_filename}
                   VERBATIM
                   COMMAND
                    ${CMAKE_COMMAND} -E make_directory ${icount_abilist_dir}
                   COMMAND
                     cat ${CMAKE_CURRENT_SOURCE_DIR}/done_abilist.txt
                         ${CMAKE_CURRENT_SOURCE_DIR}/libc_ubuntu1404_abilist.txt
                         > ${icount_abilist_filename}
                   DEPENDS done_abilist.txt libc_ubuntu1404_abilist.txt)
add_dependencies(icount icount_abilist)
install(FILES ${icount_abilist_filename}
        DESTINATION ${COMPILER_RT_INSTALL_PATH}/share)

set(ICOUNT_ABILIST_PATH ${icount_abilist_filename} CACHE INTERNAL "")
