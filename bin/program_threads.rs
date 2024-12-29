use extrae_rs::{instrument_function, profile, GlobalInfo};

#[profile]
fn myfunction(i: u32) -> u32
{
    std::thread::sleep(std::time::Duration::from_millis(10));
    i
}

fn main() -> nix::Result<()>
{
    println!("Start Program");

    let handle = std::thread::spawn(|| {
        for i in 1..10 {
            println!("hi number {} from the spawned thread!", myfunction(i));
        }
    });

    for i in 1..5 {
        println!("hi number {} from the main thread!", myfunction(i));
    }

    handle.join().unwrap();

    println!("Done");
    Ok(())
}
