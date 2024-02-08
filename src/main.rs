use std::error::Error;
use clap::{Arg, arg, ArgAction, ArgMatches};
use mconfig::mconfigurator::{MConfig, MCResult};
use std::fs::{File, read};
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let arg_matches = clap::command!()
        .arg(
            Arg::new("file")
                .long("file")
                .short('f')
                .required(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("The file to open or create."),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .action(ArgAction::SetTrue)
                .conflicts_with("key")
                .help("List the keys and values contained in the file."),
        )
        .arg(
            Arg::new("key")
                .long("key")
                .short('k')
                .help("The key to retrieve or set."),
        )
        .arg(
            Arg::new("value")
                .long("value")
                .short('v')
                .requires("key")
                .required(false)
                .help("The value to set (optional)."),
        )
        .get_matches();

    let file = arg_matches.get_one::<PathBuf>("file").expect("Required parameter 'file' is missing.");
    let data = match read(file) {
        Ok(d) => {
            println!("Loaded {} bytes from {}", d.len(), file.display());
            d
        }
        Err(e) => {
            eprintln!("Error loading {}: {}", file.display(), e);
            return Err(e.into())
        }
    };

    print!("Enter secret: ");
    std::io::stdout().flush()?;
    let mut secret = String::new();
    std::io::stdin().read_line(&mut secret)?;

    let mcnf = match MConfig::builder().load(data).secret(&secret.trim()).try_build() {
        Ok(m) => {
            println!("Loaded MConfigurator data with {} entries.", m.len());
            m
        }
        Err(e) => {
            eprintln!("Failed to load MConfigurator data: {}", e);
            return Err(e.into())
        }
    };


    if let Some(true) = arg_matches.get_one::<bool>("list") {
        for (k,v) in mcnf.iter() {
            let v = v.clone().unwrap_or("<empty>".to_string());
            println!("{k}: {v}");
        }
    }

    if let Some(key) = arg_matches.get_one::<String>("key") {
        println!("Key: {key}");
        todo!("Key handling not implemented.")
    }
    if let Some(value) = arg_matches.get_one::<String>("value") {
        println!("Value: {value}");
        todo!("Value handling not implemented")
    }

    Ok(())
}

