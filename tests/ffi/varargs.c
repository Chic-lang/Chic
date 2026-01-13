#include <stdarg.h>
#include <stdint.h>

int check_promotions(int count, ...) {
  va_list args;
  va_start(args, count);
  double f = va_arg(args, double);
  int s = va_arg(args, int);
  int b = va_arg(args, int);
  va_end(args);
  return (count == 3) && ((int)(f * 10.0) == 25) && (s == 7) && (b == 9);
}

