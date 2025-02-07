use std::process::Command;

use std::sync::Mutex;

// Static Mutex for synchronization
static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn test_program(executable: &str)
{
     let _lock = TEST_MUTEX.lock().unwrap();

    let output = Command::new(executable)
        .output()
        .expect("Failed to execute program_procedural");

    let stdout = String::from_utf8_lossy(&output.stdout);

    if !output.status.success() {
        #[cfg_attr(coverage, coverage(off))]
        {
            // Capture stdout and stderr
            println!("stdout:\n{}", stdout);

            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("stderr\n{}", stderr);
        }
    }

    assert!(output.status.success(), "program_procedural exited with an error");

    //"# Profiler TraceDir: {}", output_path.to_str().unwrap()

    //let matches = stdout.matches("# Profiler TraceDir: ").

    #[cfg(feature = "profiling")]
    assert!(stdout.contains("# Profiler TraceDir: "), "Unexpected stdout: \n---- \n{}---- \n", stdout);

    #[cfg(not(feature = "profiling"))]
    assert!(!stdout.contains("# Profiler TraceDir: "), "Unexpected stdout: \n---- \n{}---- \n", stdout);
}


#[test]
fn test_program_threads()
{
    test_program(env!("CARGO_BIN_EXE_program_threads"));
}

#[test]
fn test_program_declarative()
{
    test_program(env!("CARGO_BIN_EXE_program_declarative"));
}

#[test]
fn test_program_procedural()
{
    test_program(env!("CARGO_BIN_EXE_program_procedural"));
}

#[test]
fn test_program_tokio()
{
    test_program(env!("CARGO_BIN_EXE_program_tokio"));
}
