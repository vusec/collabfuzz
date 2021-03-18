add_llvm_library(${PASS_NAME} MODULE BBReachability.cpp)
target_compile_definitions(${PASS_NAME} PRIVATE ${LLVM_DEFINITIONS})
target_compile_features(${PASS_NAME} PRIVATE cxx_std_17)
target_include_directories(${PASS_NAME} PRIVATE ${LLVM_INCLUDE_DIRS})
target_link_libraries(${PASS_NAME} PRIVATE CollabFuzzIDAssigner::Headers
                                           nlohmann_json::nlohmann_json)

if(${CMAKE_CXX_COMPILER_ID} STREQUAL "Clang" OR ${CMAKE_CXX_COMPILER_ID}
                                                STREQUAL "GNU")
  target_compile_options(${PASS_NAME} PRIVATE "-Wall" "-Wextra"
                                              "-Wno-unused-parameter")
endif()
