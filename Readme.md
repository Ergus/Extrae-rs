# Readme

> **_NOTE:_**  This project is not totally functional yet.

This project is a sort of rust improved reimplementation of
[ExtraeWin](https://github.com/Ergus/ExtraeWin).

The main goal is to use native rust code to generate instrumentation
for highly concurrency code.

In the near future this code will add perf integration and some other stuff.

## Usage

At the moment the core includes two instrumentation methods:

### With declarative macros:

```rust

// This instruments all the function scope
fn myfunction()
{
    instrument_function!(); 
    // Some code
}

// Specifying the function name
fn myfunction2()
{
    instrument_function!("MyFunction2_manual");
    // Some code
}

// Also specify the event number
fn myfunction3()
{
    instrument_function!("MyFunction3_manual_value_20", 20);
    // Some code
}
```

### With procedural macros:

```rust

#[profile]
fn myfunction()
{
    // Some code
}

#[profile(name="MyFunction2_manual")]
fn myfunction2()
{
    // Some code
}

#[profile(name="MyFunction3_manual_value_20",value=20)]
fn myfunction3()
{
    // Some code
}
```

We provide test programs code for both cases and the user can mix both
syntaxes. 

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

