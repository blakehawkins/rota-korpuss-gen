#![recursion_limit = "1024"]
/// A generator for a simple rota using nurse data.

extern crate csv;

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate serde_derive;

extern crate serde_yaml;
extern crate structopt;

#[macro_use]
extern crate structopt_derive;

use structopt::StructOpt;

use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;

mod errors {
    error_chain! {}
}

error_chain! {
    foreign_links {
        Csv(::csv::Error);
        Io(::std::io::Error);
    }

    errors {
        InvalidMonth(m: String) {
            description("invalid month name")
            display("invalid month name: '{}'", m)
        }

        InvalidWeekday(d: String) {
            description("invalid weekday")
            display("invalid weekday name: '{}'", d)
        }

        InvalidRoom(r: String) {
            description("invalid room")
            display("invalid room name: '{}'", r)
        }

        InvalidDate(d: usize) {
            description("invalid date of the month")
            display("impossible date of the month: '{}'", d)
        }
    }
}

// use errors::*;

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
    name:    String,
    days:    Option<Vec<String>>,
    trainee: Option<bool>,
    rooms:   Option<Vec<String>>,
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
    year:      usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Config {
    people:      People,
    nurses_jobs: Vec<String>,
    rooms:       Vec<String>,
    dates:       Dates,
}

static VALID_MONTHS: [&str; 12] = ["january",
                                   "february",
                                   "march",
                                   "april",
                                   "may",
                                   "june",
                                   "july",
                                   "august",
                                   "september",
                                   "october",
                                   "november",
                                   "december"];

static VALID_WEEKDAYS: [&str; 7] = ["monday",
                                    "tuesday",
                                    "wednesday",
                                    "thursday",
                                    "friday",
                                    "saturday",
                                    "sunday"];

fn do_validate_dates(cfg: &Config) -> Result<()> {
    // Validate dates.
    // Check month.
    ensure!(VALID_MONTHS.contains(&&cfg.dates.month[..]),
            ErrorKind::InvalidMonth(cfg.dates.month.clone()));

    // Check day of week.
    ensure!(VALID_WEEKDAYS.contains(&&cfg.dates.start_day[..]),
            ErrorKind::InvalidWeekday(cfg.dates.start_day.clone()));

    // Check each nurse configured with valid dates.
    for n in cfg.people.nurses.iter().filter(|n| n.days.is_some()) {
        for d in n.days.clone().unwrap() {
            ensure!(VALID_WEEKDAYS.contains(&&d[..]),
                    ErrorKind::InvalidWeekday(d.clone()))
        }
    }

    // Check each nurse supporter configured with valid dates.
    for s in cfg.people.supporters.iter().filter(|s| s.days.is_some()) {
        for d in s.days.clone().unwrap() {
            ensure!(VALID_WEEKDAYS.contains(&&d[..]),
                    ErrorKind::InvalidWeekday(d.clone()))
        }
    }

    Ok(())
}

fn do_validate_rooms(cfg: &Config) -> Result<()> {
    for n in cfg.people.nurses.iter().filter(|n| n.rooms.is_some()) {
        for r in n.rooms.clone().unwrap() {
            ensure!(cfg.rooms.contains(&r),
                    ErrorKind::InvalidRoom(r.clone()))
        }
    }

    Ok(())
}

fn do_validates(cfg: &Config) -> Result<()> {
    do_validate_dates(&cfg)?;

    do_validate_rooms(&cfg)?;

    // Sanity check dates.
    ensure!(cfg.dates.end < 33,
            ErrorKind::InvalidDate(cfg.dates.end.clone()));

    Ok(())
}

fn write_header(wtr: &mut csv::Writer<File>, dates: &Dates) -> Result<()> {
    let mut header: Vec<String> = vec!["Name".into()];
    let offset = VALID_WEEKDAYS.iter()
                               .position(|r| r == &&dates.start_day)
                               .unwrap();
    let mut day_cycle = VALID_WEEKDAYS.iter().cycle().skip(offset);

    for i in dates.start..(dates.end + 1) {
        let day = day_cycle.next().unwrap();

        if (day != &"saturday") && (day != &"sunday") {
            header.push(format!("{} {}, {}",
                                dates.month,
                                i,
                                dates.year));
        }
    }

    wtr.write_record(&header)?;

    Ok(())
}

fn do_writes(mut wtr: &mut csv::Writer<File>, cfg: &Config) -> Result<()> {
    write_header(&mut wtr, &cfg.dates)?;

    // Nurses
    let mut nurses = HashMap::new();
    unimplemented!();

    wtr.write_record(&["Liga", "main korpuss cathlab", "main korpuss x-ray", "side korpuss cathlab"])?;

    Ok(())
}

fn run() -> Result<()> {
    let opt = Opt::from_args();

    let f = OpenOptions::new().read(true)
                              .open(&opt.input)
                              .chain_err(|| format!("Couldn't open {} as a file path.", &opt.input))?;
    let cfg: Config = serde_yaml::from_reader(f).chain_err(|| format!("The input {} was an invalid yaml file.", &opt.input))?;
    println!("{:?}", cfg);

    do_validates(&cfg)?;

    let out = OpenOptions::new().write(true)
                                .create(true)
                                .truncate(true)
                                .open(&opt.output)
                                .chain_err(|| format!("Couldn't open {} for writing.", &opt.output))?;

    let mut wtr = csv::Writer::from_writer(out);

    do_writes(&mut wtr, &cfg)?;

    wtr.flush()?;

    println!("\n  File generated successfully into {}.", &opt.output);

    Ok(())
}

fn main() {
    let mut ret = 0;

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

        ret = 1;
    }

    // Await a keyboard event before closing.
    println!("\nPress Enter when finished.");
    let _ = std::io::stdin().bytes().next().and_then(|r| r.ok());

    ::std::process::exit(ret);
}
