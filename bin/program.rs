use extrae_rs::{instrument_function, GlobalInfo, ThreadInfo, profile};

fn myfunction()
{
    instrument_function!();
    std::thread::sleep(std::time::Duration::from_millis(10));
}

fn myfunction2()
{
    instrument_function!("MyFunction");
    std::thread::sleep(std::time::Duration::from_millis(10));
}

fn myfunction3()
{
    instrument_function!("MyFunction3", 20);
    std::thread::sleep(std::time::Duration::from_millis(10));
}

#[profile]
fn myfunction4()
{
    std::thread::sleep(std::time::Duration::from_millis(10));
}

fn main() -> nix::Result<()>
{
    println!("Start Program");
    GlobalInfo::register_event_name("Event1", file!(), line!(), 10);
    ThreadInfo::emplace_event(10, 1);

    ThreadInfo::emplace_event(10, 0);

    myfunction();

    myfunction2();

    myfunction3();

    myfunction4();

    println!("Done");
    Ok(())
}