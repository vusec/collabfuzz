#include <stdio.h>
#include <stdlib.h>

#define INPUT_BUFFER_SIZE 10

int main(void) {
  char input_buffer[INPUT_BUFFER_SIZE];
  fgets(input_buffer, sizeof(input_buffer), stdin);

  int choice = strtol(input_buffer, NULL, 0);
  printf("choice: %d\n", choice);

  int value = 0;
  if (choice <= 0) {
    value = (choice * choice + 3) / 2;
  } else {
    value = choice;
  }

  if (value != 42) {
    printf("%d\n", value);
  }

  return EXIT_SUCCESS;
}
