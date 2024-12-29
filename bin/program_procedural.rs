use extrae_rs::{instrument_function, GlobalInfo, ThreadInfo, profile};

#[profile]
fn myfunction()
{
    std::thread::sleep(std::time::Duration::from_millis(10));
}

#[profile(name="MyFunction2_manual")]
fn myfunction2()
{
    std::thread::sleep(std::time::Duration::from_millis(10));
}

#[profile(name="MyFunction3_manual_value_20",value=20)]
fn myfunction3() -> u32
{
    std::thread::sleep(std::time::Duration::from_millis(10));
    0
}

fn main() -> nix::Result<()>
{
    println!("Start Program");

   myfunction();

    myfunction2();

    let _ = myfunction3();

    println!("Done");
    Ok(())
}
