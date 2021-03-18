cargo_build(NAME ${RTLIB_NAME})

install(PROGRAMS $<TARGET_FILE:${RTLIB_NAME}>
        DESTINATION ${CMAKE_INSTALL_LIBDIR})

# Create custom ABI list
if(NOT DFSAN_ORIGINAL_ABILIST)
  set(DFSAN_ORIGINAL_ABILIST
      ${LLVM_LIBRARY_DIRS}/clang/${LLVM_PACKAGE_VERSION}/share/dfsan_abilist.txt
  )

  if(NOT EXISTS ${DFSAN_ORIGINAL_ABILIST})
    message(STATUS "Could not find abilist in: ${DFSAN_ORIGINAL_ABILIST}")

    # Fedora stores dfsan_abilist.txt outside of the "share" folder
    set(DFSAN_ORIGINAL_ABILIST
        ${LLVM_LIBRARY_DIRS}/clang/${LLVM_PACKAGE_VERSION}/dfsan_abilist.txt
    )
    if(NOT EXISTS ${DFSAN_ORIGINAL_ABILIST})
      message(FATAL_ERROR "Could not find abilist in: ${DFSAN_ORIGINAL_ABILIST}")
    endif()
  endif()

  message(STATUS "Original DFSan ABI list: ${DFSAN_ORIGINAL_ABILIST}")
endif()

set(CUSTOM_ABILIST_NAME input_bytes_tracer_abilist.txt)
add_custom_command(
  OUTPUT ${CUSTOM_ABILIST_NAME}
  COMMAND
    Python3::Interpreter ${PROJECT_SOURCE_DIR}/scripts/generate_abilist.py
    ${DFSAN_ORIGINAL_ABILIST} "${CMAKE_CURRENT_SOURCE_DIR}/project_abilist.txt"
    -o ${CUSTOM_ABILIST_NAME}
  COMMENT "Generating custom ABI list"
  DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/project_abilist.txt")

add_custom_target(${PROJECT_NAME}_abilist ALL DEPENDS ${CUSTOM_ABILIST_NAME})

set(${PROJECT_NAME}_ABILIST_PATH
    "${CMAKE_CURRENT_BINARY_DIR}/${CUSTOM_ABILIST_NAME}"
    CACHE INTERNAL "")

install(FILES ${${PROJECT_NAME}_ABILIST_PATH}
        DESTINATION ${CMAKE_INSTALL_DATADIR})

# XXX: Unfortunately, it appears there is no sensible way to extract the
# dependencies from the target
set(${PROJECT_NAME}_RTLIB_DEPS
    ""
    CACHE INTERNAL "")

# These symbols have to be wrapped using ld because they are present in
# compiler-rt as well
set(${PROJECT_NAME}_DFSAN_RT_OVERRIDE
    "__dfsw_fgets" "__dfsw_read" "__dfsw_pread"
    CACHE INTERNAL "")
