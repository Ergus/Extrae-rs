# Readme

This project is a sort of rust improved reimplementation of
[ExtraeWin](https://github.com/Ergus/ExtraeWin). The main goal is to
use native rust code to generate instrumentation for highly
concurrency code. In a format compatible with the
[Paraver][https://tools.bsc.es/paraver] flexible performance analysis
tool.

For more complex or distributed systems (specially outside the Rust
world) and HPC I strongly recommend using
[Extrae](https://tools.bsc.es/extrae) instead of this.

The current implementation is intended to work almost lock-free and
with minimal overhead for traces generation.  Currently the code needs
to be instrumented with some of the methods described in the
[Usage](#usage) section.

There are pending features in the [TODO](#todo) section but the
project is functional AS IS. I am open to new features and features
requests with a compromise to include them if they fit in the project
objective, are intended to be really useful and don't imply any extra
performance impact/overhead or excessive complexity.

## Usage

At the moment the core includes 3 instrumentation methods. Using a
single methods to instrument is generally preferred (the best
instrumentation is the one that does not bother your code
logic). However, the three methods can be freely mixed if the user
wants to.

### With declarative macros:

```rust

// This instruments all the function scope
fn myfunction()
{
    instrument_function!();
    // Some code

	// This is a scope to be instrumented automatically
	{
		instrument_scope!("MyScope");
		// Some code
	}
}

// Specifying the function name to show in the trace
fn myfunction2()
{
    instrument_function!("MyFunction2_manual");
    // Some code

	// This is a scope to be instrumented with a custom value
	{
		instrument_scope!("MyScope2");
		// Some code
		instrument_update!(10); // Can use any possitive value > 1
	}

}

// Also specify the event number, this is usefull when we want to enforce
// events numbers to create fancy paraver events filters.
fn myfunction3()
{
    instrument_function!("MyFunction3_manual_value_20", 20);
    // Some code
}
```

### With procedural macros:

With procedural macros we can instrument complete functions more
easily.  However to instrument scopes we still need to use declarative
macros.


```rust

// This instruments all the function scope
#[extrae_profile]
fn myfunction()
{
    // Some code
}

// Specifying the function name to show in the trace
#[extrae_profile(name="MyFunction2_manual")]
fn myfunction2()
{
    // Some code
}

// Also specify the event number, this is usefull when we want to enforce
// events numbers to create fancy paraver events filters.
#[extrae_profile(name="MyFunction3_manual_value_20",value=20)]
fn myfunction3()
{
    // Some code
}
```

### Tokio integration.

Tokio already has imtegration with the
[tracing](https://crates.io/crates/tracing) crate. While the method it
uses is a bit different in philosophy, Extrae-rs provides out of the
box integration and some extra features.


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

### Trace directory format

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

### Paraver basics description

Paraver events have 2 basic parameters: `id` and `value`.

For functions and scoped events the `id` is internally assigned by the
profiler during the execution. A `value=1` indicate the event is
starting in that point and `value=0` indicates the end of the
event.

The user don't need to emit those values manually and the provided
macros automate these values.

Any `value > 1` can be emitted with the `instrument_update` macro in
order to have more detailed information of the function's parts.

When enables, other events like performance counters are emitted
together with the instrumented ones.

You can find a set of [Paraver
tutorials](https://tools.bsc.es/paraver-tutorials) from their
creators. And a useful video introduction to
[Paraver](https://www.youtube.com/watch?v=R8_EhVpOzb0)


```shell
./target/debug/visualizer TRACEDIR_1735338966/Trace_1.bin
```

## TODO

1. OTF2 format generator. To use with vampir and other profilers.
2. CTF format. There are some nice tools to work with this format, but
   nothing that paraver can't do with a right config.  Any way I
   consider this because it is intended to be very simple to
   implement.
3. Add compatibility with the
   [profiling](https://crates.io/crates/profiling) crate. This works
   almost exactly like this project, but with different macro names.
