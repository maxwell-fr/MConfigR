use clap::{Arg, ArgAction};
use mconfig::MConfig;
use std::error::Error;
use std::fs::{write, read};
use std::io::Write;
use std::path::{PathBuf};

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
        .arg(
            Arg::new("empty")
                .long("empty")
                .short('e')
                .requires("key")
                .conflicts_with("value")
                .action(ArgAction::SetTrue)
                .help("Specify the key should be created with no value."),
        )
        .arg(
            Arg::new("remove")
                .long("remove")
                .short('r')
                .requires("key")
                .conflicts_with("value")
                .action(ArgAction::SetTrue)
                .help("Delete the specified key and value, if any."),
        )
        .get_matches();

    let file = arg_matches
        .get_one::<PathBuf>("file")
        .expect("Required parameter 'file' is missing.");
    let data = match read(file) {
        Ok(d) => {
            println!("Loaded {} bytes from {}", d.len(), file.display());
            d
        }
        Err(e) => {
            eprintln!("Error loading {}: {}", file.display(), e);
            return Err(e.into());
        }
    };

    // Retrieve secret from stdin. Todo: make this hide the typed text visually
    print!("Enter secret: ");
    std::io::stdout().flush()?;
    let mut secret = String::new();
    std::io::stdin().read_line(&mut secret)?;

    let mut mcnf = match MConfig::builder()
        .load(data)
        .secret(&secret.trim())
        .try_build()
    {
        Ok(m) => {
            println!("Loaded MConfigurator data with {} entries.", m.len());
            m
        }
        Err(e) => {
            eprintln!("Failed to load MConfigurator data: {}", e);
            return Err(e.into());
        }
    };

    // listing objects, nothing else
    if arg_matches.get_flag("list") {
        for (k, v) in mcnf.iter() {
            let v = v.clone().unwrap_or("<empty>".to_string());
            println!("{k}: {v}");
        }
    }

    // The key argument is mutex with list
    if let Some(key) = arg_matches.get_one::<String>("key") {
        if arg_matches.get_flag("remove") {
            let old = mcnf.remove(key);
            if let Some(old) = old {
                println!("Removed {key} with value {}", old.unwrap_or("<empty>".to_string()));
                write(file, mcnf.to_vec())?;
                println!("Updated {}", file.display());
            }

        } else if arg_matches.get_flag("empty") {
            let old = mcnf.try_insert(key.clone(), None)?;
            write(file, mcnf.to_vec())?;
            println!("Added empty {key}. Previous value: {}", old.unwrap_or("n/a".to_string()));
        }
        else if let Some(value) = arg_matches.get_one::<String>("value") {
            let old = mcnf.try_insert(key.clone(), Some(value.clone()))?;
            write(file, mcnf.to_vec())?;
            println!("Added value {value} to  key {key}. Previous value: {}", old.unwrap_or("n/a".to_string()));
        } else {
            if let Some(value) = mcnf.get(key) {
                let value = value.clone().unwrap_or("<empty>".to_string());
                println!("{key}: {value}");
            } else {
                println!("{key} not found.");
            }
        }
    }

    Ok(())
}
