include(GNUInstallDirs)

set(SYMLINK_NAME "clang_icount")

set(PASS_INSTALL_PREFIX "${CMAKE_INSTALL_PREFIX}/lib${LLVM_LIBDIR_SUFFIX}")
set(PASS_PATH
    "${PASS_INSTALL_PREFIX}/$<TARGET_FILE_NAME:LLVMInstructionCounter>")

set(RTLIB_PATH "${CMAKE_INSTALL_PREFIX}/lib/linux/libclang_rt.icount-x86_64.a")
set(RTLIB_DEPS "${${PROJECT_NAME}_RTLIB_DEPS}")
set(ABILIST_PATH "${CMAKE_INSTALL_PREFIX}/share/icount_abilist.txt")

set(WRAPPER_NAME "clang_icount_wrapper.py")
configure_file(${WRAPPER_NAME}.in ${WRAPPER_NAME}.gen @ONLY)
file(
  GENERATE
  OUTPUT ${WRAPPER_NAME}
  INPUT "${CMAKE_CURRENT_BINARY_DIR}/${WRAPPER_NAME}.gen")

# These symlinks need to use a relative path, so they should always be installed
# in the same directory as the wrapper script.
add_custom_target(c_symlink ALL
  ${CMAKE_COMMAND} -E
    create_symlink ${WRAPPER_NAME} ${CMAKE_CURRENT_BINARY_DIR}/${SYMLINK_NAME}
  BYPRODUCTS ${SYMLINK_NAME})
add_custom_target(cxx_symlink ALL
  ${CMAKE_COMMAND} -E
    create_symlink ${WRAPPER_NAME} ${CMAKE_CURRENT_BINARY_DIR}/${SYMLINK_NAME}++
  BYPRODUCTS ${SYMLINK_NAME}++)

install(PROGRAMS
          ${CMAKE_CURRENT_BINARY_DIR}/${WRAPPER_NAME}
          ${CMAKE_CURRENT_BINARY_DIR}/${SYMLINK_NAME}
          ${CMAKE_CURRENT_BINARY_DIR}/${SYMLINK_NAME}++
        DESTINATION ${CMAKE_INSTALL_BINDIR})
