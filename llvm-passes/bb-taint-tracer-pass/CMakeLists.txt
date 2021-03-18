cmake_minimum_required(VERSION 3.13)

project(BBTaintTracer)

list(APPEND CMAKE_MODULE_PATH "${PROJECT_SOURCE_DIR}/cmake/")

enable_language(Rust)
include(CMakeCargo)

include(GNUInstallDirs)

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

find_package(Python3 REQUIRED)

set(PASS_NAME "LLVM${PROJECT_NAME}")
set(RTLIB_NAME "${PROJECT_NAME}RT")

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
