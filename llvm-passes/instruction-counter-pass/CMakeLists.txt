cmake_minimum_required(VERSION 3.14)

project(InstructionCounter)

find_package(LLVM 9 REQUIRED CONFIG)
message(STATUS "Found LLVM ${LLVM_PACKAGE_VERSION}")
message(STATUS "Using LLVMConfig.cmake in: ${LLVM_DIR}")

list(APPEND CMAKE_MODULE_PATH "${LLVM_CMAKE_DIR}")
include(AddLLVM)

add_subdirectory(Pass)

set(COMPILER_RT_STANDALONE_BUILD TRUE)
set(COMPILER_RT_INSTALL_PATH "." CACHE PATH
    "Path where built compiler-rt libraries should be installed.")
add_subdirectory(Runtime)

# Unfortunately, there is no way to generate these flags
set(${PROJECT_NAME}_RTLIB_DEPS "-ldl -lpthread -lrt -lc" CACHE INTERNAL "")

add_subdirectory(Utils)

if(BUILD_TESTING)
  find_package(Python3 REQUIRED)
  find_program(LIT_PROGRAM lit)

  if(${LIT_PROGRAM} STREQUAL "LIT_PROGRAM-NOTFOUND")
    message(
      WARNING "Command 'lit' not found. You will not be able to run tests.")
  else()
    add_subdirectory(Tests)
  endif()
endif()

if(CMAKE_SOURCE_DIR STREQUAL CMAKE_CURRENT_SOURCE_DIR)
  add_subdirectory(packaging)
endif()
