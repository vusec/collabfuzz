set(ICOUNT_RESOURCE_DIR ${CMAKE_INSTALL_PREFIX} CACHE PATH
   "Path to the folder containing the resources")
set(ICOUNT_BLACKLIST_NAME "icount_abilist.txt" CACHE STRING
   "Name of the icount blacklist file")
configure_file(icount_config.h.in icount_config.h @ONLY)

set(PASS_NAME "LLVMInstructionCounter")
add_llvm_library(${PASS_NAME} MODULE InstructionCounterPass.cpp)
target_compile_definitions(${PASS_NAME} PRIVATE ${LLVM_DEFINITIONS})
target_include_directories(${PASS_NAME}
   PRIVATE ${LLVM_INCLUDE_DIRS} ${CMAKE_CURRENT_BINARY_DIR})

if(${CMAKE_CXX_COMPILER_ID} STREQUAL "Clang" OR
   ${CMAKE_CXX_COMPILER_ID} STREQUAL "GNU")
   target_compile_options(
      ${PASS_NAME} PRIVATE "-Wall" "-Wextra" "-Wno-unused-parameter")
endif()

add_custom_target(pass DEPENDS ${PASS_NAME})

add_library(CollabFuzzInstructionCounter::${PASS_NAME} ALIAS ${PASS_NAME})
