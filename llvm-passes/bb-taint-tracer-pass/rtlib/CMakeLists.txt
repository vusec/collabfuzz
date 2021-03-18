add_subdirectory(runtime_rs)
add_library(${RTLIB_NAME} SHARED io_wrappers.c)

if(${CMAKE_C_COMPILER_ID} STREQUAL "Clang" OR ${CMAKE_C_COMPILER_ID}
                                                STREQUAL "GNU")
  target_compile_options(${RTLIB_NAME} PRIVATE "-Wall" "-Wextra")
endif()

set_target_properties(${RTLIB_NAME} PROPERTIES POSITION_INDEPENDENT_CODE ON)

target_link_libraries(${RTLIB_NAME} PRIVATE bb_taint_tracer_runtime_rs)

install(
  TARGETS ${RTLIB_NAME}
  LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
  ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})

# Create custom ABI list

if(NOT DFSAN_ORIGINAL_ABILIST)
  # XXX: Probably broken if multiple versions of clang are installed
  file(GLOB_RECURSE DFSAN_ORIGINAL_ABILIST_LIST
       ${LLVM_LIBRARY_DIRS}/clang/*/dfsan_abilist.txt)
  list(GET DFSAN_ORIGINAL_ABILIST_LIST 0 DFSAN_ORIGINAL_ABILIST)
  if("${DFSAN_ORIGINAL_ABILIST}" STREQUAL "")
    message(FATAL_ERROR "Could not find dfsan_abilist.txt")
  endif()

  message(STATUS "Original DFSan ABI list: ${DFSAN_ORIGINAL_ABILIST}")
endif()

add_custom_command(
  OUTPUT bb_taint_tracer_abilist.txt
  COMMAND
    Python3::Interpreter ${PROJECT_SOURCE_DIR}/scripts/generate_abilist.py
    ${DFSAN_ORIGINAL_ABILIST} "${CMAKE_CURRENT_SOURCE_DIR}/project_abilist.txt"
    -o bb_taint_tracer_abilist.txt
  COMMENT "Generating custom ABI list"
  DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/project_abilist.txt")

add_custom_target(${PROJECT_NAME}_abilist ALL
                  DEPENDS bb_taint_tracer_abilist.txt)

set(${PROJECT_NAME}_ABILIST_PATH
    "${CMAKE_CURRENT_BINARY_DIR}/bb_taint_tracer_abilist.txt"
    CACHE INTERNAL "")

install(FILES ${${PROJECT_NAME}_ABILIST_PATH}
        DESTINATION ${CMAKE_INSTALL_DATADIR})

# XXX: Unfortunately, it appears there is no sensible way to extract the
# dependencies from the target
set(${PROJECT_NAME}_RTLIB_DEPS "" CACHE INTERNAL "")

# These symbols have to be wrapped using ld because they are present in
# compiler-rt as well
set(${PROJECT_NAME}_DFSAN_RT_OVERRIDE
    "__dfsw_fgets" "__dfsw_read" "__dfsw_pread"
    CACHE INTERNAL "")
