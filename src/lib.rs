mod bufferinfo;
pub use bufferinfo::BufferInfo;

mod buffer;

mod nameset;
mod bufferset;

mod global_info;
pub use global_info::GlobalInfo;

mod thread_info;
pub use thread_info::ThreadInfo;

mod profiler;
pub use profiler::Guard;

mod parser;
pub(crate) use parser::Merger;

mod subscriber;
pub use subscriber::ExtraeSubscriber;

#[macro_export]
macro_rules! instrument_function {
    () => {
        #[cfg(feature = "profiling")]
        let _guard = {
            static PROFILER_ONCE: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
            extrae_rs::Guard::new(
                *PROFILER_ONCE.get_or_init(|| extrae_rs::GlobalInfo::register_event_name(
                    {
                        fn f() {}
                        fn type_name_of<T>(_: T) -> &'static str {
                            std::any::type_name::<T>()
                        }
                        let name = type_name_of(f);
                        // 16 is the length of ::{{closure}}::f
                        &name[..name.len() - 16]
                    },
                    Some(file!()), Some(line!()), None)),
                1
            )
        };
    };
    ($arg1:literal) => {
        #[cfg(feature = "profiling")]
        let _guard = {
            // Create a profiler guard
            static PROFILER_ONCE: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
            extrae_rs::Guard::new(
                *PROFILER_ONCE.get_or_init(|| extrae_rs::GlobalInfo::register_event_name($arg1, Some(file!()), Some(line!()), None)),
                1
            )
        };
    };
    ($arg1:literal, $arg2:literal) => {
        #[cfg(feature = "profiling")]
        let _guard = {
            // Create a profiler guard
            static PROFILER_ONCE: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
            extrae_rs::Guard::new(
                *PROFILER_ONCE.get_or_init(|| extrae_rs::GlobalInfo::register_event_name($arg1, Some(file!()), Some(line!()), Some($arg2))),
                1
            )
        };
    };
}

#[macro_export]
macro_rules! instrument_update {
    ($arg1:expr) => {
        #[cfg(feature = "profiling")]
        _guard.update($arg1);
    };
}

// Re-export the macro. This is essential for users of your library
pub use extrae_macros::extrae_profile;
