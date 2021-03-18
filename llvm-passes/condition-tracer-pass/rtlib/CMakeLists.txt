add_library(${RTLIB_NAME} RuntimeLibrary.cpp Tracer.cpp)
target_compile_features(${RTLIB_NAME} PRIVATE cxx_std_17)
target_link_libraries(${RTLIB_NAME}
                      PRIVATE GSL Boost::program_options -lstdc++fs)

if(${CMAKE_CXX_COMPILER_ID} STREQUAL "Clang"
   OR ${CMAKE_CXX_COMPILER_ID} STREQUAL "GNU")
  target_compile_options(${RTLIB_NAME} PRIVATE "-Wall" "-Wextra")
endif()

set_target_properties(${RTLIB_NAME} PROPERTIES POSITION_INDEPENDENT_CODE ON)
install(TARGETS ${RTLIB_NAME}
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
        ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})
