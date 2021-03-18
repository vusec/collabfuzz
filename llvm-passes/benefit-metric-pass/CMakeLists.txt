cmake_minimum_required(VERSION 3.13)

project(BBReachability)

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

set(PASS_NAME "LLVM${PROJECT_NAME}")

add_subdirectory(external)
add_subdirectory(bb_reachability)
