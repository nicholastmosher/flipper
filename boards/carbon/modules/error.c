#define __private_include__
#include <flipper/error.h>

#ifdef __use_error__
/* Define the virtual interface for this module. */
const struct _error error = {
	error_configure,
	error_pause,
	error_resume,
	error_raise,
	error_get,
	error_clear,
};
#endif