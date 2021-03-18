cmake_minimum_required(VERSION 3.13)

project(EdgeTracer)

include(GNUInstallDirs)
include(CheckIPOSupported)

check_ipo_supported(RESULT ipo_supported OUTPUT ipo_output)
if(NOT ipo_supported)
  message(WARNING "Could not enable IPO: ${ipo_output}")
  message(WARNING "IPO was NOT enabled, performance may be reduced.")
endif()

if(NOT LLVM_FOUND)
  find_package(LLVM 9 REQUIRED CONFIG)
  message(STATUS "Found LLVM ${LLVM_PACKAGE_VERSION}")
  message(STATUS "Using LLVMConfig.cmake in: ${LLVM_DIR}")
  list(APPEND CMAKE_MODULE_PATH "${LLVM_CMAKE_DIR}")
  include(AddLLVM)
endif()

if(NOT TARGET CollabFuzzIDAssigner::LLVMIDAssigner
   OR NOT TARGET CollabFuzzIDAssigner::Headers)
  message(
    STATUS "CollabFuzzIDAssigner does not exist, search using find_package")
  find_package(CollabFuzzIDAssigner REQUIRED)
else()
  message(STATUS "CollabFuzzIDAssigner already exists")
endif()

set(Boost_USE_STATIC_LIBS ON)
find_package(Boost REQUIRED COMPONENTS program_options)

set(PASS_NAME "LLVM${PROJECT_NAME}")
set(RTLIB_NAME "${PROJECT_NAME}RT")

# XXX: Unfortunately, it appears there is no sensible way to extract the
# dependencies from the target
set(${PROJECT_NAME}_RTLIB_DEPS
    "-l:libboost_program_options.a -lstdc++ -lstdc++fs"
    CACHE INTERNAL "")

add_subdirectory(external)
add_subdirectory(src)
add_subdirectory(rtlib)

if(BUILD_TESTING)
  find_program(LIT_PROGRAM lit)

  if(${LIT_PROGRAM} STREQUAL "LIT_PROGRAM-NOTFOUND")
    message(
      WARNING "Command 'lit' not found. You will not be able to run tests.")
  else()
    add_subdirectory(test)
  endif()
endif()

if(BUILD_BENCHMARK)
  find_package(Python3 REQUIRED)

  if(NOT ${CMAKE_C_COMPILER_ID} STREQUAL "Clang")
    message(WARNING "LLVM passes cannot be applied without clang. "
                    "Set CMAKE_C_COMPILER to clang for benchmark.")
  else()
    add_subdirectory(benchmark)
  endif()
endif()
