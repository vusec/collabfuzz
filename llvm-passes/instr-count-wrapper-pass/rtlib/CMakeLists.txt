add_subdirectory(runtime_rs)
add_library(${RTLIB_NAME} SHARED io_wrappers.c)

# Unfortunately LLVM does not use Modern CMake patterns, so to check if this is
# subproject together with InstructionCounter, we need to check if the
# corresponding variables are set.
if(COMPILER_RT_OUTPUT_DIR)
  add_dependencies(${RTLIB_NAME} compiler-rt-headers)
  target_include_directories(${RTLIB_NAME} PRIVATE
    ${COMPILER_RT_OUTPUT_DIR}/include)
else()
  target_include_directories(${RTLIB_NAME} PRIVATE
    ${CMAKE_INSTALL_PREFIX}/include)
endif()

if(${CMAKE_C_COMPILER_ID} STREQUAL "Clang" OR ${CMAKE_C_COMPILER_ID}
                                                STREQUAL "GNU")
  target_compile_options(${RTLIB_NAME} PRIVATE "-Wall" "-Wextra")
endif()

set_target_properties(${RTLIB_NAME} PROPERTIES POSITION_INDEPENDENT_CODE ON)

target_link_libraries(${RTLIB_NAME} PRIVATE inst_count_wrapper_runtime_rs)

install(
  TARGETS ${RTLIB_NAME}
  LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
  ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})

set(${PROJECT_NAME}_ABILIST_PATH
    "${CMAKE_CURRENT_SOURCE_DIR}/icount_wrapper_abilist.txt"
    CACHE INTERNAL "")

install(FILES ${${PROJECT_NAME}_ABILIST_PATH}
        DESTINATION ${CMAKE_INSTALL_DATADIR})

# XXX: Unfortunately, it appears there is no sensible way to extract the
# dependencies from the target
set(${PROJECT_NAME}_RTLIB_DEPS "" CACHE INTERNAL "")
