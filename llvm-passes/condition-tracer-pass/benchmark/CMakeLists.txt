# In order to add a new benchmark, it is sufficient to append its bitcode file
# to PROGS and add its command line arguments in a new <prog-name>_ARGS
# variable. Additional dependencies can be appended at the end of this file,
# targeting the <prog-name>_instr target.

set(PROGS objdump.bc md5sum.bc)
set(objdump_ARGS -d /bin/ls)
set(md5sum_ARGS /bin/ls)

if(NOT TARGET benchmark)
  # This target may be created by the project using this one, so check before
  # trying to create it.
  add_custom_target(benchmark)
endif()

# Prepare common compile options
set(OPT_LOAD_ASSIGNER "-load"
                      $<TARGET_FILE:CollabFuzzIDAssigner::LLVMIDAssigner>)
set(OPT_LOAD_TRACER "-load" $<TARGET_FILE:${PASS_NAME}>)

set(OPT_COMPILE_OPTIONS)
foreach(ITER IN LISTS OPT_LOAD_ASSIGNER OPT_LOAD_TRACER)
  list(APPEND OPT_COMPILE_OPTIONS "-Xclang" "${ITER}")
endforeach()
list(JOIN OPT_COMPILE_OPTIONS " " OPT_COMPILE_OPTIONS_STR)

# Iterate over benchmark programs
foreach(PROG IN ITEMS ${PROGS})
  # Setting the LANGUAGE to C makes the compiler behave correctly with bitcode
  set_property(SOURCE ${PROG} PROPERTY LANGUAGE C)

  get_filename_component(PROG_NAME ${PROG} NAME_WE)
  set(PROG_INSTR "${PROG_NAME}_instr_${PROJECT_NAME}")

  add_executable(${PROG_INSTR} EXCLUDE_FROM_ALL ${PROG})
  add_dependencies(${PROG_INSTR} ${PASS_NAME} ${RTLIB_NAME})

  # The SHELL option avoids the de-duplication of the options
  target_compile_options(${PROG_INSTR}
                         PRIVATE "-O0" "SHELL:${OPT_COMPILE_OPTIONS_STR}")

  target_link_libraries(${PROG_INSTR} PRIVATE ${RTLIB_NAME})

  set(PROG_ARGS ${${PROG_NAME}_ARGS})
  configure_file(benchmark.py.in ${PROG_NAME}_benchmark.py.gen @ONLY)
  file(
    GENERATE
    OUTPUT ${PROG_NAME}_benchmark.py
    INPUT "${CMAKE_CURRENT_BINARY_DIR}/${PROG_NAME}_benchmark.py.gen")

  add_custom_target(${PROJECT_NAME}-benchmark-${PROG_NAME} Python3::Interpreter
                    ${PROG_NAME}_benchmark.py DEPENDS ${PROG_INSTR})
  add_dependencies(benchmark ${PROJECT_NAME}-benchmark-${PROG_NAME})
endforeach()

# Additional per program dependencies
target_link_libraries(objdump_instr_${PROJECT_NAME} PRIVATE -ldl)
