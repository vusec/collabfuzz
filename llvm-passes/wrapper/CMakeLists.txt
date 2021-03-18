include(GNUInstallDirs)

set(PASS_INSTALL_PREFIX "${CMAKE_INSTALL_PREFIX}/lib${LLVM_LIBDIR_SUFFIX}")
set(RTLIB_INSTALL_PREFIX "${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_LIBDIR}")

set(ID_ASSIGNER_PASS_PATH
    "${PASS_INSTALL_PREFIX}/$<TARGET_FILE_NAME:LLVMIDAssigner>")
set(EDGE_TRACER_PASS_PATH
    "${PASS_INSTALL_PREFIX}/$<TARGET_FILE_NAME:LLVMEdgeTracer>")
set(CONDITION_TRACER_PASS_PATH
    "${PASS_INSTALL_PREFIX}/$<TARGET_FILE_NAME:LLVMConditionTracer>")
set(BB_REACHABILITY_PASS_PATH
    "${PASS_INSTALL_PREFIX}/$<TARGET_FILE_NAME:LLVMBBReachability>")
set(BB_TAINT_TRACER_PASS_PATH
    "${PASS_INSTALL_PREFIX}/$<TARGET_FILE_NAME:LLVMBBTaintTracer>")
set(INSTRUCTION_COUNT_PASS_PATH
    "${PASS_INSTALL_PREFIX}/$<TARGET_FILE_NAME:LLVMInstructionCounter>")
set(INST_COUNT_WRAPPER_PASS_PATH
    "${PASS_INSTALL_PREFIX}/$<TARGET_FILE_NAME:LLVMInstCountWrapper>")
set(STATIC_BRANCH_METRICS_PASS_PATH
    "${PASS_INSTALL_PREFIX}/$<TARGET_FILE_NAME:LLVMStaticMetrics>")
set(INPUT_BYTES_TRACER_PASS_PATH
    "${PASS_INSTALL_PREFIX}/$<TARGET_FILE_NAME:LLVMInputBytesTracer>")

set(EDGE_TRACER_RTLIB_PATH
    "${RTLIB_INSTALL_PREFIX}/$<TARGET_FILE_NAME:EdgeTracerRT>")
set(EDGE_TRACER_RTLIB_DEPS ${EdgeTracer_RTLIB_DEPS})
set(CONDITION_TRACER_RTLIB_PATH
    "${RTLIB_INSTALL_PREFIX}/$<TARGET_FILE_NAME:ConditionTracerRT>")
set(CONDITION_TRACER_RTLIB_DEPS ${ConditionTracer_RTLIB_DEPS})
set(BB_TAINT_TRACER_RTLIB_PATH
    "${RTLIB_INSTALL_PREFIX}/$<TARGET_FILE_NAME:BBTaintTracerRT>")
set(BB_TAINT_TRACER_RTLIB_DEPS ${BBTaintTracer_RTLIB_DEPS})

get_filename_component(BB_TAINT_TRACER_ABILIST_NAME
                       "${BBTaintTracer_ABILIST_PATH}" NAME)
set(BB_TAINT_TRACER_ABILIST_PATH
    "${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_DATADIR}/${BB_TAINT_TRACER_ABILIST_NAME}"
)
set(BB_TAINT_TRACER_DFSAN_RT_OVERRIDE "${BBTaintTracer_DFSAN_RT_OVERRIDE}")

# This pass does not have a nice interface to extract this information
set(INSTRUCTION_COUNT_RTLIB_PATH
    "${CMAKE_INSTALL_PREFIX}/lib/linux/libclang_rt.icount-x86_64.a")
set(INSTRUCTION_COUNT_RTLIB_DEPS "-ldl -lpthread -lrt")
set(INSTRUCTION_COUNT_ABILIST_PATH
    "${CMAKE_INSTALL_PREFIX}/share/icount_abilist.txt")

set(INST_COUNT_WRAPPER_RTLIB_PATH
    "${RTLIB_INSTALL_PREFIX}/$<TARGET_FILE_NAME:InstCountWrapperRT>")
set(INST_COUNT_WRAPPER_RTLIB_DEPS ${InstCountWrapper_RTLIB_DEPS})

get_filename_component(INST_COUNT_WRAPPER_ABILIST_NAME
                       "${InstCountWrapper_ABILIST_PATH}" NAME)
set(INST_COUNT_WRAPPER_ABILIST_PATH
    "${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_DATADIR}/${INST_COUNT_WRAPPER_ABILIST_NAME}"
)

set(INPUT_BYTES_TRACER_RTLIB_PATH
    "${RTLIB_INSTALL_PREFIX}/$<TARGET_FILE_NAME:InputBytesTracerRT>")

get_filename_component(INPUT_BYTES_TRACER_ABILIST_NAME
                       "${InputBytesTracer_ABILIST_PATH}" NAME)
set(INPUT_BYTES_TRACER_ABILIST_PATH
  "${CMAKE_INSTALL_PREFIX}/${CMAKE_INSTALL_DATADIR}/${INPUT_BYTES_TRACER_ABILIST_NAME}"
)

configure_file(collab_fuzz_wrapper.in collab_fuzz_wrapper.gen)
file(
  GENERATE
  OUTPUT collab_fuzz_wrapper
  INPUT "${CMAKE_CURRENT_BINARY_DIR}/collab_fuzz_wrapper.gen")
install(PROGRAMS ${CMAKE_CURRENT_BINARY_DIR}/collab_fuzz_wrapper
        DESTINATION ${CMAKE_INSTALL_BINDIR})

if(BUILD_TESTING)
  find_package(Python3 3.7 REQUIRED COMPONENTS Interpreter)

  set(ID_ASSIGNER_PASS_PATH $<TARGET_FILE:LLVMIDAssigner>)
  set(EDGE_TRACER_PASS_PATH $<TARGET_FILE:LLVMEdgeTracer>)
  set(CONDITION_TRACER_PASS_PATH $<TARGET_FILE:LLVMConditionTracer>)
  set(BB_REACHABILITY_PASS_PATH $<TARGET_FILE:LLVMBBReachability>)
  set(BB_TAINT_TRACER_PASS_PATH $<TARGET_FILE:LLVMBBTaintTracer>)
  set(INSTRUCTION_COUNT_PASS_PATH $<TARGET_FILE:LLVMInstructionCounter>)
  set(INST_COUNT_WRAPPER_PASS_PATH $<TARGET_FILE:LLVMInstCountWrapper>)
  set(STATIC_BRANCH_METRICS_PASS_PATH $<TARGET_FILE:LLVMStaticMetrics>)
  set(INPUT_BYTES_TRACER_PASS_PATH $<TARGET_FILE:LLVMInputBytesTracer>)

  set(EDGE_TRACER_RTLIB_PATH $<TARGET_FILE:EdgeTracerRT>)
  set(CONDITION_TRACER_RTLIB_PATH $<TARGET_FILE:ConditionTracerRT>)
  set(BB_TAINT_TRACER_RTLIB_PATH $<TARGET_FILE:BBTaintTracerRT>)
  set(INSTRUCTION_COUNT_RTLIB_PATH
      "${COMPILER_RT_OUTPUT_DIR}/lib/linux/libclang_rt.icount-x86_64.a")
  set(INST_COUNT_WRAPPER_RTLIB_PATH $<TARGET_FILE:InstCountWrapperRT>)
  set(INPUT_BYTES_TRACER_RTLIB_PATH $<TARGET_FILE:InputBytesTracerRT>)

  set(BB_TAINT_TRACER_ABILIST_PATH "${BBTaintTracer_ABILIST_PATH}")
  set(INSTRUCTION_COUNT_ABILIST_PATH "${ICOUNT_ABILIST_PATH}")
  set(INST_COUNT_WRAPPER_ABILIST_PATH "${InstCountWrapper_ABILIST_PATH}")
  set(INPUT_BYTES_TRACER_ABILIST_PATH "${InputBytesTracer_ABILIST_PATH}")

  configure_file(collab_fuzz_wrapper.in wrapper_test.gen)
  file(
    GENERATE
    OUTPUT wrapper_test
    INPUT "${CMAKE_CURRENT_BINARY_DIR}/wrapper_test.gen")

  add_custom_target(
    ${PROJECT_NAME}-check
    COMMAND Python3::Interpreter ${CMAKE_CURRENT_BINARY_DIR}/wrapper_test
            /tmp/outdir ${CMAKE_CURRENT_SOURCE_DIR}/test_assets/objdump.bc --
            -ldl
    COMMAND rm -r /tmp/outdir
    DEPENDS ${CMAKE_CURRENT_BINARY_DIR}/wrapper_test
            LLVMIDAssigner
            LLVMEdgeTracer
            LLVMConditionTracer
            LLVMBBReachability
            LLVMBBTaintTracer
            LLVMInstructionCounter
            LLVMInstCountWrapper
            LLVMStaticMetrics
            LLVMInputBytesTracer
            EdgeTracerRT
            ConditionTracerRT
            BBTaintTracerRT
            BBTaintTracer_abilist
            compiler-rt # Runtime library for InstructionCounter
            InstCountWrapperRT
            InputBytesTracerRT
            InputBytesTracer_abilist
            )

  if(NOT TARGET check)
    # This target may have been created by the parent project
    add_custom_target(check)
  endif()
  add_dependencies(check ${PROJECT_NAME}-check)
endif()
