mod buffer;
pub use buffer::BufferInfo;

mod nameset;
mod bufferset;

mod global_info;
pub use global_info::GlobalInfo;

mod thread_info;
pub use thread_info::ThreadInfo;

mod profiler;
pub use profiler::Guard;

#[macro_export]
macro_rules! instrument_function {
    () => {
        // Create a profiler guard
        let _guard = {
            static PROFILER_ONCE: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
            extrae_rs::Guard::new(
                *PROFILER_ONCE.get_or_init(|| GlobalInfo::register_event_name(
                    {
                        fn f() {}
                        fn type_name_of<T>(_: T) -> &'static str {
                            std::any::type_name::<T>()
                        }
                        let name = type_name_of(f);
                        // 16 is the length of ::{{closure}}::f
                        &name[..name.len() - 16]
                    },
                    file!(), line!(), 0)),
                1
            )
        };
    };
    ($arg1:expr) => {
        let _guard = {
            // Create a profiler guard
            static PROFILER_ONCE: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
            extrae_rs::Guard::new(
                *PROFILER_ONCE.get_or_init(|| GlobalInfo::register_event_name($arg1, file!(), line!(), 0)),
                1
            )
        };
    };
    ($arg1:expr, $arg2:expr) => {
        let _guard = {
            // Create a profiler guard
            static PROFILER_ONCE: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
            extrae_rs::Guard::new(
                *PROFILER_ONCE.get_or_init(|| GlobalInfo::register_event_name($arg1, file!(), line!(), $arg2)),
                1
            )
        };
    };
}

// Re-export the macro. This is essential for users of your library
pub use extrae_macros::profile;
