# Readme

> **_NOTE:_**  This project is not totally functional yet.

This project is a sort of rust improved reimplementation of
[ExtraeWin](https://github.com/Ergus/ExtraeWin).

The main goal is to use native rust code to generate instrumentation
for highly concurrency code.

## Usage

At the moment the core includes 3 instrumentation methods:

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

#[extrae_profile]
fn myfunction()
{
    // Some code
}

#[extrae_profile(name="MyFunction2_manual")]
fn myfunction2()
{
    // Some code
}

#[extrae_profile(name="MyFunction3_manual_value_20",value=20)]
fn myfunction3()
{
    // Some code
}
```

### Tokio integration.

```rust

#[tracing::instrument]
async fn task1() {
    info!(value = 5, "Task 1 started");
    time::sleep(Duration::from_millis(500)).await;
 }

#[tokio::main]
async fn main() {

    // Set up a subscriber that logs to stdout
    let subscriber = ExtraeSubscriber::new();
    set_global_default(subscriber).expect("Could not set global default subscriber");

    // Run tasks concurrently
    let handle1 = task::spawn(task1());

    let handle2 = task::spawn(task1());

    // Wait for both tasks to complete
    let _ = tokio::join!(handle1, handle2);
}

```

We provide test programs code for all cases and the user can mix
syntaxes.


```shell
cargo expand --bin program --features profiling
```

## Perf integration

The code includes perf events integration and supports the following

Hardware:

	`cycles`, `instructions`, `cache-references`, `cache-misses`,
	`branch-instructions`, `branch-misses`, `bus-cycles`,
	`stalled-cycles-frontend`, `stalled-cycles-backend`, `ref-cpu-cycles`,
	`page-faults`, `context-switches`, `cpu-migrations`,
	`page-faults-min`, `page-faults-maj`.

Software:

	`page-faults` `context-switches` `cpu-migrations` `page-faults-min`
	`page-faults-maj`.

The events are tracked per thread.

The user can specify the desired events with 2 methods:

1. Environment variables:

```bash
EXTRAE_COUNTERS="cycles,cache-misses" ./target/debug/program
```

2. With a config file. Create a toml file in the execution directory
   with the entry:

```toml
counters = ["cycles", "cache-misses"]
```

and execute the program as usual:

```bash
./target/debug/program
```

The profiler initialization informs about the enabled counters.
When both methods are enables, the environment variable has priority.

## Trace generation

Without the `--features profiling` here (or when importing the crate)
no instrumentation will be generated at all.

After the execution of instrumented code the profiler creates a new
directory `TRACE_[suffix]`. The configuration option `suffix` can be
used to specify the `[suffix]`.

```bash
EXTRAE_SUFFIX="mysuffix" ./target/debug/program
```

This option can be specified in a configuration file equivalent see
[Perf integration](#perf-integration).

By default the suffix is the timespamp of the execution unix time. An
error will be triggered if the directory `TRACE_[suffix]` already
exist.

The profiler prints the name of the directory at the end of the
execution, which is useful when the directory name is auto-generated.

### Trace direcotry format

The trace directory directory contains multiple trace files:
`Trace_[tid].bin`. There is one of such files for every thread created
during the program execution, where `[tid]` is the internal thread
id. The main thread is always 1.

The `Trace_*.bin` files are binary files with all the trace events. The
final trace needs to merge all the files in order to visualize them
with tools like [Paraver](https://tools.bsc.es/paraver).

There are other two files: `Trace.pcf`, `Trace.row` needed by paraver
format.

For development purposes we provide a `visualizer` executable that can
be used to read the binary trace file as plain text.

For example:

```shell
./target/debug/visualizer TRACEDIR_1735338966/Trace_1.bin
```

