#ifndef __cpu_h__
#define __cpu_h__

/* Include all types and macros exposed by the Flipper Toolbox. */
#include <flipper/libflipper.h>

/* Declare the virtual interface for this module. */
extern const struct _cpu {
	int (* configure)(void);
	void (* reset)(void);
	void (* cycle)(void);
	void (* halt)(void);
	void (* power)(uint8_t power);
	lf_error_t (* dfu)(void);
} cpu;

#ifdef __private_include__

/* Declare the _lf_module structure for this module. */
extern struct _lf_module _cpu;

/* Declare the FMR overlay for this module. */
enum { _cpu_configure, _cpu_reset, _cpu_cycle, _cpu_halt, _cpu_power, _cpu_dfu };

/* Declare the prototypes for all of the functions within this module. */
extern int cpu_configure(void);
extern void cpu_reset(void);
extern void cpu_cycle(void);
extern void cpu_halt(void);
extern void cpu_power(uint8_t power);
extern lf_error_t cpu_dfu(void);

#endif
#endif
