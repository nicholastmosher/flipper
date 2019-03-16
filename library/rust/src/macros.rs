/// Generates Rust wrapper implementations of Flipper modules.
///
/// A Flipper function invocation is simply a description of a function to execute. Namely, an
/// invocation contains the following information:
///
///   1) The name of the module which the function belongs to.
///   2) The index of the function within the module.
///   3) The values and types of the arguments to the function.
///   4) The expected return type of the function.
///
/// The implementation of any remote invocation can be directly derived from the signature
/// of the function being executed. This macro accepts a description of the function index and
/// signature as input and generates the implementation for performing remote invocation as output.
///
/// Below is an example demonstrating how one would generate the implementation for the LED module.
///
/// ```rust-norun
/// flipper_module! (led::Led: [
///     0 => fn led_configure() -> LfType::lf_void,
///     1 => fn led_rgb(red: u8, green: u8, blue: u8) -> LfType::lf_void,
/// ]);
/// ```
///
/// This macro would generate the following code:
///
/// ```rust-norun
/// pub mod led {
///     use flipper::{Flipper, Client, Args, LfType, Result};
///
///     pub trait Led: Client {
///         fn led_configure() -> Result<u64> {
///             let mut args = Args::new();
///             self.invoke("led", 0, LfType::lf_void, &args)
///         }
///
///         fn led_rgb(red: u8, green: u8, blue: u8) -> Result<u64> {
///             let mut args = Args::new();
///             let mut args = args.append(red);
///             let mut args = args.append(blue);
///             let mut args = args.append(green);
///             self.invoke("led", 1, LfType::lf_void, &args)
///         }
///     }
/// }
/// ```
///
/// A consumer of the generated module could then use it like this:
///
/// ```rust-norun
/// use flipper::Flipper;
/// use led::Led;
///
/// let flipper: Flipper = unimplemented!();
/// flipper.led_configure();
/// flipper.led_rgb(10, 10, 0);
/// ```
#[macro_export]
macro_rules! flipper_module (
    ($ns:ident :: $name:ident: [
        $(
            $idx:expr => fn $func:ident ( $($args:tt)* ) -> $lfret:expr
        ),*$(,)* ]
    ) => {
        pub mod $ns {
            use $crate::{Flipper, Client, Args, LfType, Result};
            pub trait $name: Client {
                $(
                    fn $func (&mut self, $($args)*) -> Result<u64> {
                        __flipper_module_func_impl!(self, stringify!($ns), $idx, $lfret, $($args)*)
                    }
                )*
            }

            impl<T> $name for T where T: Client { }
        }
    }
);

#[doc(hidden)]
#[macro_export]
macro_rules! __flipper_module_func_impl (
    ($self_:ident, $key:expr, $idx:expr, $lfret:expr, $($name:ident: $typ:ty),*$(,)*) => {{
        let mut args = Args::new();
        $(
            let mut args = args.append($name);
        )*
        $self_.invoke($key, $idx, $lfret, &args)
    }}
);
