use std::fs::File;
use std::io::Write;
use std::path::Path;
use mconfig;

fn main() {
    let (secret, mc_vec) = mconfig::demo();

    let path = Path::new("mconfig_demo.dat");
    let display = path.display();

    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    let r = file.write_all(mc_vec.as_ref());
    println!("Secret: {secret}  File: {display}  Result: {:?}", r);

}

