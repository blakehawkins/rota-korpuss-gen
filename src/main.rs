#![recursion_limit = "1024"]
/// A generator for a simple rota using nurse data.
// Needs:
//    - Validate date strings
//    - Validate room-required found in rooms list
//    - Sanity check dates
//    - Validate works fine for unicode

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate serde_derive;

extern crate serde_yaml;
extern crate structopt;

#[macro_use]
extern crate structopt_derive;

use structopt::StructOpt;

use std::fs::OpenOptions;
use std::io::Read;

mod errors {
    error_chain!{}
}

use errors::*;

#[derive(StructOpt, Debug)]
#[structopt(name = "Rota Korpuss Gen", about = "Generate a rota for Stradini")]
struct Opt {
    #[structopt(help = "Input file", default_value = "config.yaml")]
    input: String,

    #[structopt(help = "Output file", default_value = "rota.csv")]
    output: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Nurse {
    name: String,
    days: Option<Vec<String>>,
    trainee: Option<bool>,
    rooms: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Supporter {
    name: String,
    days: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct People {
    nurses:     Vec<Nurse>,
    supporters: Vec<Supporter>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Dates {
    month:     String,
    start_day: String,
    start:     usize,
    end:       usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Config {
    people:      People,
    nurses_jobs: Vec<String>,
    rooms:       Vec<String>,
    dates:       Dates,
}

fn run() -> Result<()> {
    let opt = Opt::from_args();

    let f = OpenOptions::new().read(true)
                              .open(&opt.input)
                              .chain_err(|| format!("Couldn't open {} as a file path.", &opt.input))?;
    let cfg: Config = serde_yaml::from_reader(f).chain_err(|| format!("The input {} was an invalid yaml file.", &opt.input))?;
    println!("{:?}", cfg);

    Ok(())
}

fn main() {
    if let Err(ref e) = run() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }

        // The backtrace is not always generated. Try to run this example with
        // `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
        }
    }

    // Await a keyboard event before closing.
    println!("\nPress any key when finished.");
    let _ = std::io::stdin().bytes().next().and_then(|r| r.ok());

    ::std::process::exit(1);
}
