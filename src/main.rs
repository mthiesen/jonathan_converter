use clap::{Command, Arg};
use std::io::prelude::*;

fn pause() {
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    write!(stdout, "Press return to continue...").unwrap();
    stdout.flush().unwrap();

    let _ = stdin.read(&mut [0u8]).unwrap();
}

fn main() {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(Arg::new("DIRECTORY")
            .help("The root directory of the 'Jonathan' game. By default the current directory is used.")
            .index(1))
        .after_help("The PCX files in the GRAFIK directory are converted to PNG files and written to the new directory GRAFIK_PNG.\n\
                     The TCT files in the TEXT directory are converted to UTF-8 text files and written to the new directory TEXT_TXT.")
        .get_matches();

    println!(concat!(
        env!("CARGO_PKG_NAME"),
        " ",
        env!("CARGO_PKG_VERSION")
    ));
    println!(env!("CARGO_PKG_AUTHORS"));
    println!();

    if let Err(ref err) = jonathan_converter::run(matches.value_of("DIRECTORY").unwrap_or(".")) {
        eprintln!("{err:#}");
        pause();
        std::process::exit(1);
    } else {
        pause();
    }
}
