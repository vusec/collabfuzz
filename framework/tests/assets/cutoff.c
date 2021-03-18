#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define INPUT_BUFFER_SIZE 10

int main(void) {
  char input_buffer[INPUT_BUFFER_SIZE];
  fgets(input_buffer, sizeof(input_buffer), stdin);

  int choice = atoi(input_buffer);
  printf("choice: %d\n", choice);

  if (choice == 1) {
    puts("inside first if");

    if (input_buffer[1] == 'a') {
      puts("inside second if");
    }
  }

  return EXIT_SUCCESS;
}
