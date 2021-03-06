#include "libflipper.h"

enum { _button_read, _button_configure };

uint8_t button_read(void);
int button_configure(void);

void *button_interface[] = { &button_read, &button_configure };

LF_MODULE(button, "button", button_interface);

LF_WEAK uint8_t button_read(void) {
    lf_return_t retval;
    lf_invoke(lf_get_selected(), "button", _button_read, lf_int8_t, &retval, NULL);
    return (uint8_t)retval;
}

LF_WEAK int button_configure(void) {
    lf_return_t retval;
    lf_invoke(lf_get_selected(), "button", _button_configure, lf_int_t, &retval, NULL);
    return (int)retval;
}
