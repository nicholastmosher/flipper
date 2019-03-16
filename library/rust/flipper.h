#include <stdint.h>
#include <stdlib.h>
#include <stdbool.h>

enum LfResult {
  lf_success = 0,
  lf_null_pointer = 1,
  lf_invalid_string = 2,
  lf_package_not_loaded = 3,
  lf_no_devices_found = 4,
  lf_index_out_of_bounds = 5,
  lf_illegal_type = 6,
  lf_invocation_error = 7,
  lf_illegal_handle = 8,
};
typedef uint32_t LfResult;

enum LfType {
  lf_void = 2,
  lf_int = 4,
  lf_ptr = 6,
  lf_uint8 = 0,
  lf_uint16 = 1,
  lf_uint32 = 3,
  lf_uint64 = 7,
  lf_int8 = 8,
  lf_int16 = 9,
  lf_int32 = 11,
  lf_int64 = 15,
};
typedef uint8_t LfType;

typedef uint64_t LfValue;

typedef uint8_t LfFunction;

/*
 * Appends a new argument (value and type) onto an existing argument list.
 *
 * If the argument being appended is smaller than 8 bytes, then its value
 * should be initialized using its native type initialization, then cast into
 * a `LfValue` when passed to this function.
 *
 * ```c
 * void *argv;
 * lf_create_args(&argv);
 *
 * uint32_t argument1 = 0x40000000;
 * LfType arg1kind = lf_uint32;
 *
 * lf_append_arg(argv, (LfValue) argument1, arg1kind);
 * ```
 *
 * As new items are appended to the list, the list will automatically re-alloc
 * itself and grow as necessary.
 *
 * If the value passed for `kind` is not valid (i.e. not defined in the LfType
 * enum), then nothing will be appended to the list, and an `LfResult` of
 * `lf_illegal_type` will be returned.
 */
LfResult lf_append_arg(void *argv, LfValue value, LfType kind);

/*
 * Returns an opaque pointer to a list of Flipper devices and the length of
 * the list.
 *
 * There are no guarantees about the representation of the device list. The
 * returned value should be used solely as a handle to provide to other
 * functions that accept a Flipper list.
 *
 * The pointer returned as `devices` is heap-allocated and owned by the caller.
 * The proper way to release the device list is by using
 * `lf_release(devices)`.
 */
LfResult lf_attach_usb(void **devices, uint32_t *length);

/*
 * Creates an empty argument list to be used with `lf_invoke`.
 *
 * This function creates an opaque, heap-allocated struct used for preparing a
 * remote function call to Flipper. The typical usage is to create the argument
 * list, then to append each argument to it using `lf_append_arg`, then to pass
 * it to `lf_invoke` to perform the invocation.
 *
 * Since the list is heap-allocated, it is the responsibility of the caller to
 * free the memory when the list is no longer needed. The proper way to do this
 * is by using `lf_release`.
 *
 * # Example
 *
 * Here's an example of building an argument list using `lf_create_args` and
 * `lf_append_arg`:
 *
 * ```c
 * void *argv = NULL;
 * LfResult result = lf_create_args(&argv);
 *
 * // Result will be nonzero if there is an error.
 * if (result) {
 * printf("There was an error creating an argument list!\n");
 * return 1;
 * }
 *
 * // Add a uint8_t of value 10 as the first argument. See `lf_append_arg`.
 * lf_append_arg(argv, (LfValue) 10, lf_uint8);
 *
 * // Release the argument list when you're done with it.
 * lf_release(argv);
 * ```
 */
LfResult lf_create_args(void **argv);

/*
 * Executes a remote function on the given Flipper device.
 *
 * Flipper invocations are composed of 4 things:
 *
 * 1) The name of the module where the function to execute is defined
 * 2) The index of the function to execute within its parent module
 * 3) The list of argument values and types to be passed to the function
 * 4) The expected return type that should be produced by executing the
 * function
 *
 * To send an invocation, we must also provide the handle of the device to
 * send to, and the address of a variable to store the return value.
 *
 * # Example
 *
 * Consider the built-in "led" module, which controls Flipper's onboard RGB
 * led. The primary function in this module is
 * `led_rgb(uint8_t red, uint8_t green, uint8_t blue)`, which is the
 * first function in the module (located at index 0).
 *
 * In order to invoke `led_rgb(10, 20, 30)` in C, we would do the following:
 *
 * ```c
 * // Get the list of Flipper: Carbon USB devices
 * void *usb_devices;
 * uint32_t length;
 * lf_attach_usb(&usb_devices, &length);
 *
 * // Select the first Flipper device in the list
 * void *flipper = lf_select(usb_devices, 0);
 *
 * // Construct the argument list
 * void *args;
 * lf_create_args(&args);
 *
 * uint8_t red = 10, green = 20, blue = 30;
 * lf_append_arg(args, (LfValue) red, lf_uint8);
 * lf_append_arg(args, (LfValue) green, lf_uint8);
 * lf_append_arg(args, (LfValue) blue, lf_uint8);
 *
 * // Send the invocation and read the result
 * LfValue result;
 * lf_invoke(flipper, "led", 0, args, lf_void, &result);
 *
 * // Release the argument list, selected Flipper, and usb list
 * lf_release(args);
 * lf_release(flipper);
 * lf_release(usb_devices);
 * ```
 */
LfResult lf_invoke(void *device,
                   const char *module,
                   LfFunction function,
                   const void *argv,
                   LfType return_type,
                   LfValue *return_value);

LfResult lf_release(void *argv);

/*
 * Retrieves a device from the device list at the given index. Index 0 is the
 * first device.
 *
 * The returned handle represents a single attached Flipper device. This
 * handle is only valid while the device list it came from is valid. That is,
 * if `lf_release(devices)` is called, then the Flipper handle that was
 * returned by this function is no longer valid (but still must be freed).
 * Handles returned by `lf_select` must be freed using `lf_release`.
 *
 * If the given devices pointer is NULL, then NULL is returned.
 *
 * If the given index is out of bounds for the device list, then NULL is
 * returned.
 */
LfResult lf_select(void *devices, uint32_t index, void **device);
