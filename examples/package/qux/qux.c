#define __private_include__
#include <qux/qux.h>
#include <flipper/atsam4s/atsam4s.h>

const char _fmr_module_name[] __attribute__((section (".name"))) = "qux";

int qux_configure(void) {
	PIOA -> PIO_PER = PIO_PA8;
	PIOA -> PIO_OER = PIO_PA8;
	PIOA -> PIO_OWER = PIO_PA8;
	PIOA -> PIO_ODSR |= PIO_PA8;
	return lf_success;
}

void qux_test(void) {

}
