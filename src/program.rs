use extrae_rs::{GlobalInfo, ThreadInfo};

fn main() -> nix::Result<()>
{
    println!("Start Program");
    GlobalInfo::register_event_name("Event1", file!(), line!(), 10);
    ThreadInfo::emplace_event(10, 1);

    ThreadInfo::emplace_event(10, 0);

    println!("Done");
    Ok(())
}
