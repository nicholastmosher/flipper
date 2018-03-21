#include <flipper.h>

struct _lf_module *lf_module_create(char *name, int idx) {
	lf_assert(name, failure, E_NULL, "No name provided to '%s'.", __PRETTY_FUNCTION__);
	lf_assert(strlen(name) < 16, failure, E_OVERFLOW, "Module name '%s' is invalid. Module names must be 16 characters or less.", name);
	struct _lf_module *module = calloc(1, sizeof(struct _lf_module));
	lf_assert(module, failure, E_MALLOC, "Failed to allocate memory for new _lf_module.");
	size_t len = strlen(name) + 1;
	module->name = malloc(len);
	memcpy(&module->name, name, len);
	module->idx = idx;
	return module;
failure:
	return NULL;
}

int lf_module_release(struct _lf_module *module) {
	lf_assert(module, failure, E_NULL, "NULL module provided to '%s'.", __PRETTY_FUNCTION__);
	free(module->name);
	free(module);
	return lf_success;
failure:
	return lf_error;
}
