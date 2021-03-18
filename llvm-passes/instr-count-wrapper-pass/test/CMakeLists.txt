set(IDASSIGNER_PASS_PATH $<TARGET_FILE:CollabFuzzIDAssigner::LLVMIDAssigner>)

if(COMPILER_RT_OUTPUT_DIR)
  set(ICOUNT_PASS_PATH $<TARGET_FILE:CollabFuzzInstructionCounter::LLVMInstructionCounter>)
  set(ICOUNT_RTLIB_PATH ${COMPILER_RT_OUTPUT_DIR}/lib/linux/libclang_rt.icount-x86_64.a)
else()
  set(ICOUNT_PASS_PATH ${CMAKE_INSTALL_PREFIX}/lib${LLVM_LIBDIR_SUFFIX}/LLVMInstructionCounter.so)
  set(ICOUNT_RTLIB_PATH ${CMAKE_INSTALL_PREFIX}/lib/linux/libclang_rt.icount-x86_64.a)
  set(ICOUNT_ABILIST_PATH ${CMAKE_INSTALL_PREFIX}/share/icount_abilist.txt)
endif()

set(ICOUNT_RTLIB_DEPS "-ldl -lpthread -lrt")

set(PASS_PATH $<TARGET_FILE:${PASS_NAME}>)
set(RTLIB_PATH $<TARGET_FILE:${RTLIB_NAME}>)
set(RTLIB_DEPS ${${PROJECT_NAME}_RTLIB_DEPS})
set(RTLIB_ABILIST_PATH ${${PROJECT_NAME}_ABILIST_PATH})
configure_file(lit.cfg.in lit.cfg.gen @ONLY)
file(
  GENERATE
  OUTPUT lit.cfg
  INPUT "${CMAKE_CURRENT_BINARY_DIR}/lit.cfg.gen")

add_custom_target(
  ${PROJECT_NAME}-check
  COMMAND ${LIT_PROGRAM} ${CMAKE_CURRENT_BINARY_DIR} -sv
  DEPENDS ${PASS_NAME} ${RTLIB_NAME})

if(NOT TARGET check)
  # This target may have been created by the parent project
  add_custom_target(check)
endif()
add_dependencies(check ${PROJECT_NAME}-check)
