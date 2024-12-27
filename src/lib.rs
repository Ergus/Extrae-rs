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
        static PROFILER_FUNCTION_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
        let _guard = extrae_rs::Guard::new(
            *PROFILER_FUNCTION_ID.get_or_init(|| GlobalInfo::register_event_name("Function", file!(), line!(), 0)),
            1
        );
    };

    ($arg1:expr) => {
        // Create a profiler guard
        static PROFILER_FUNCTION_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
        let _guard = extrae_rs::Guard::new(
            *PROFILER_FUNCTION_ID.get_or_init(|| GlobalInfo::register_event_name($arg1, file!(), line!(), 0)),
            1
        );
    };

    ($arg1:expr, $arg2:expr) => {
        // Create a profiler guard
        static PROFILER_FUNCTION_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
        let _guard = extrae_rs::Guard::new(
            *PROFILER_FUNCTION_ID.get_or_init(|| GlobalInfo::register_event_name($arg1, file!(), line!(), $arg2)),
            1
        );
    };
    
}
