#include <flipper.h>

int main(void) {
    void *device = carbon_attach();
    carbon_select_u2(device);
    led_rgb(10, 10, 0);

    return 0;
}