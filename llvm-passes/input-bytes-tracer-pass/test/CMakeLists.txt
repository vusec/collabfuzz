set(IDASSIGNER_PASS_PATH $<TARGET_FILE:CollabFuzzIDAssigner::LLVMIDAssigner>)
set(PASS_PATH $<TARGET_FILE:${PASS_NAME}>)
set(RTLIB_PATH $<TARGET_FILE:${RTLIB_NAME}>)
set(RTLIB_DEPS ${${PROJECT_NAME}_RTLIB_DEPS})
set(GENERATED_ABILIST_PATH ${${PROJECT_NAME}_ABILIST_PATH})
configure_file(lit.cfg.in lit.cfg.gen @ONLY)
file(
  GENERATE
  OUTPUT lit.cfg
  INPUT "${CMAKE_CURRENT_BINARY_DIR}/lit.cfg.gen")

add_custom_target(
  ${PROJECT_NAME}-check
  COMMAND ${LIT_PROGRAM} ${CMAKE_CURRENT_BINARY_DIR} -sv
  DEPENDS ${PASS_NAME} ${RTLIB_NAME} ${PROJECT_NAME}_abilist)

if(NOT TARGET check)
  # This target may have been created by the parent project
  add_custom_target(check)
endif()
add_dependencies(check ${PROJECT_NAME}-check)
