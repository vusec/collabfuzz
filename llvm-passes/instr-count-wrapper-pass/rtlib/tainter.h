#include <sanitizer/icount_interface.h>
#include <stdbool.h>

void tainter_trace_open(int fd, const char *file_path);
void tainter_trace_close(int fd);
bool tainter_is_input_fd(int fd);
bool tainter_is_debug_enabled();
icount_shadow tainter_get_init_count();
