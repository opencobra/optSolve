/* Runner self-test (feature 005, task T003): a program that writes to both standard streams, so
 * the shared runner's no-output assertion is shown to reject it. */
#include <stdio.h>
int main(void) {
    fputs("to standard output\n", stdout);
    fputs("to standard error\n", stderr);
    return 0;
}
