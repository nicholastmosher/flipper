#include <flipper/libflipper.h>

#ifdef __use_task__
#define __private_include__
#include <flipper/task.h>

LF_MODULE(_task, "task", "Pause, resume, or stop tasks running in the scheduler.");

/* Define the virtual interface for this module. */
const struct _task task = {
	os_task_pause,
	os_task_resume,
	os_task_stop
};

LF_WEAK int os_task_pause(int pid) {
	return lf_invoke(&_task, _task_pause, fmr_args(fmr_infer(pid)));
}

LF_WEAK int os_task_resume(int pid) {
	return lf_invoke(&_task, _task_resume, fmr_args(fmr_infer(pid)));
}

LF_WEAK int os_task_stop(int pid) {
	return lf_invoke(&_task, _task_stop, fmr_args(fmr_infer(pid)));
}

#endif