cmake_minimum_required(VERSION 3.13)

project(AnalysisPasses VERSION 0.1.1)

include(CheckIPOSupported)

find_package(LLVM 9 REQUIRED CONFIG)
message(STATUS "Found LLVM ${LLVM_PACKAGE_VERSION}")
message(STATUS "Using LLVMConfig.cmake in: ${LLVM_DIR}")
list(APPEND CMAKE_MODULE_PATH "${LLVM_CMAKE_DIR}")
include(AddLLVM)

check_ipo_supported(RESULT ipo_supported OUTPUT ipo_output)
if(NOT ipo_supported)
  message(WARNING "Could not enable IPO: ${ipo_output}")
  message(WARNING "IPO was NOT enabled, performance may be reduced.")
endif()

# XXX: instruction-counter-pass creates the check CMake custom target, so it
# should be added first. This target is added by CMake code in LLVM, so it
# cannot be changed easily.
add_subdirectory(instruction-counter-pass)

add_subdirectory(id-assigner-pass)
add_subdirectory(edge-tracer-pass)
add_subdirectory(condition-tracer-pass)
add_subdirectory(benefit-metric-pass)
add_subdirectory(bb-taint-tracer-pass)
add_subdirectory(instr-count-wrapper-pass)
add_subdirectory(static-branch-metrics-pass)
add_subdirectory(input-bytes-tracer-pass)

add_subdirectory(wrapper)

# Sensible default generators
set(CPACK_GENERATOR                 "STGZ;TGZ")
# Escape CPACK_ variables; defaults to FALSE for backwards compatibility
set(CPACK_VERBATIM_VARIABLES        TRUE)
include(CPack)
