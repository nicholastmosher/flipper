#include <flipper.h>

extern struct _lf_module usb;

LF_FUNC("usb") int usb_configure(void) {
	dyld_register(&THIS_DEVICE, &usb);
	printf("Configured USB.\n");
	return lf_success;
}
