# Readme

> **_NOTE:_**  This project is not totally functional yet.

This project is a sort of rust improved reimplementation of
[ExtraeWin](https://github.com/Ergus/ExtraeWin).

The main goal is to use native rust code to generate instrumentation
for highly concurrency code.

In the near future this code will add perf integration and some other stuff.

## Usage

At the moment the core includes multiple instrumentation methods:

```rust
use extrae_rs::{instrument_function, GlobalInfo, ThreadInfo, profile};

// Instrument all the function scope
#[profile]
fn myfunction()
{
    std::thread::sleep(std::time::Duration::from_millis(10));
}

// This is almost equivalent to the code above
fn myfunction()
{
    instrument_function!(); // This instruments all the function scope
    std::thread::sleep(std::time::Duration::from_millis(10));
}

// Similar, but specifying the function name
fn myfunction2()
{
    instrument_function!("MyFunction2_manual");
    std::thread::sleep(std::time::Duration::from_millis(10));
}

// Also specify the user event number
fn myfunction3()
{
    instrument_function!("MyFunction3_manual_value_20", 20);
    std::thread::sleep(std::time::Duration::from_millis(10));
}
```

The test program can be executed as:

```shell
cargo expand --bin program --features profiling
```

Without the `--features profiling` here (or when importing the crate)
no instrumentation will be generated at all.

After the execution of instrumented code the profiler creates a new
`TRACE\_[timestamp]` directory. It contains multiple trace files:
`Trace_#`. There is one of such files for every thread created during
the program execution, where `#` is the internal thread id. The main
thread is always 1.

The profiler prints the name of the directory at the end of the
execution.

The `Trace_#` files are binary files with all the trace events. The
final trace needs to merge all the files in order to visualize them
with tools like [Paraver](https://tools.bsc.es/paraver).

There are other two files: `Trace.pcf`, `Trace.row` needed by paraver
format.

For development purposes we provide a `visualizer` executable that can
be used to read the binary trace file as plain text.

For example:

```shell
./target/debug/visualizer TRACEDIR_1735338966/Trace_1
```

