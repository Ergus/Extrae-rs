use extrae_rs::profile;

#[profile]
fn myfunction1(i: u32) -> u32
{
    std::thread::sleep(std::time::Duration::from_millis(10));
    i
}

#[profile]
fn myfunction2(i: u32) -> u32
{
    std::thread::sleep(std::time::Duration::from_millis(10));
    i
}

fn main() -> nix::Result<()>
{
    println!("Start Program");

    std::thread::scope(|s| {
        for thread in 1..4 {
            s.spawn(move || {
                for i in 1..5 {
                    println!("Thread: {} function1!: {}", thread, myfunction1(i));
                }
                for i in 1..5 {
                    println!("Thread: {} function2!: {}", thread, myfunction2(i));
                }
            });
        }
    });

    println!("hello from the main thread");

    for i in 1..10 {
        println!("Call function1!: {}", myfunction1(i));
    };

    println!("Done");
    Ok(())
}
