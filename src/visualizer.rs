use std::env;

use extrae_rs::BufferInfo;

fn main() -> nix::Result<()>
{
    let args: Vec<String> = env::args().collect();
    dbg!(&args);
    let tracepath = std::path::PathBuf::from(&args[1]);

    println!("Start visualizer");

    let mut file = std::fs::File::open(&tracepath).unwrap();
    let imported_info = BufferInfo::from_file(&mut file);

    print!("{}", imported_info);

    println!("Done");
    Ok(())
}
