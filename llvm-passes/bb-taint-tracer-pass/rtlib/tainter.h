#include <sanitizer/dfsan_interface.h>
#include <stdbool.h>

void tainter_trace_open(int fd, const char *file_path);
void tainter_trace_close(int fd);
bool tainter_is_input_fd(int fd);
bool tainter_is_debug_enabled();
dfsan_label tainter_get_input_label();
