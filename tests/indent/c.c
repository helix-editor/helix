#include <stdio.h>

struct Point {
  int x;
  int y;
};

enum Color { RED, GREEN, BLUE };

int compute(int a, int b) {
  int values[3] = {1, 2, 3};
  for (int i = 0; i < 3; i++) {
    if (values[i] > a) {
      printf("%d\n", values[i]);
    } else {
      a += values[i];
    }
  }
  while (a > 0)
    a--;
  if (a > 5)
    a -= 5;
  for (int i = 0; i < b; i++)
    a += i;
  do
    a++;
  while (a < 100);
  switch (b) {
  case 1:
    return a;
  case 2:
    return b;
  default:
    return 0;
  }
}

int long_function_name(int the_first_argument, int the_second_argument,
                       int the_third_argument, int the_fourth_argument) {
  return compute(the_first_argument + the_second_argument,
                 the_third_argument + the_fourth_argument);
}

union Value {
  int i;
  float f;
};

void process(int *data, int len) {
  int matrix[2][2] = {
    {1, 2},
    {3, 4},
  };

  int i = 0;
  do {
    data[i] = i;
    i++;
  } while (i < len);

  for (i = 0; i < len; i++) {
    switch (data[i]) {
    case 0:
      continue;
    default:
      break;
    }
  }

  if (len < 0)
    goto done;

  done:
  return;
}
