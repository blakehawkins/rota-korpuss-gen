#![recursion_limit = "1024"]
/// A generator for a simple rota using nurse data.

use anyhow::{ensure, Context, Result};
use itertools::Itertools;
use serde::{Serialize, Deserialize};
use clap::Parser;
use thiserror::Error;

use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::iter;


#[derive(Error, Debug, Clone, Eq, PartialEq)]
enum ErrorKind {
    #[error("Invalid month name: {0}")]
    InvalidMonth(String),

    #[error("Invalid weekday name: {0}")]
    InvalidWeekday(String),

    #[error("Invalid date of the month: {0}")]
    InvalidDate(usize),

    #[error("Invalid room name: {0}")]
    InvalidRoom(String),
}      

#[derive(Parser, Debug)]
#[command(name = "Rota Korpuss Gen", about = "Generate a rota for Stradini")]
struct Opt {
    #[arg(help = "Input file", default_value = "config.yaml")]
    input: String,

    #[arg(help = "Output file", default_value = "rota.csv")]
    output: String,

    #[arg(help = "Don't block before exit", short='b', long="no-block")]
    block: bool,
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
struct NursesJob {
    name:         String,
    for_trainees: Option<bool>
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct Config {
    people:       People,
    nurses_jobs:  Vec<NursesJob>,
    rooms:        Vec<String>,
    dates:        Dates,
    excel:        Option<bool>,
    job_room_sep: Option<String>,
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

fn maybe_write_excel_sep<W: std::io::Write>(wtr: &mut W,
                                            cfg: &Config) -> Result<()> {
    if cfg.excel.is_some() && cfg.excel.unwrap() != false {
        writeln!(wtr, "sep=,")?;
    }

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


fn write_nurses(wtr: &mut csv::Writer<File>, cfg: &Config) -> Result<usize> {
    // Nurses
    let mut nurse_map: HashMap<&str, Vec<String>> = HashMap::new();
    cfg.people.nurses.iter().for_each(|n| {
        nurse_map.insert(&n.name, vec![]);
    });

    let offset = VALID_WEEKDAYS.iter()
                               .position(|r| r == &&cfg.dates.start_day)
                               .unwrap();
    let mut day_cycle = VALID_WEEKDAYS.iter().cycle().skip(offset);

    let people_count = cfg.people.nurses.len();

    (cfg.dates.start..(cfg.dates.end + 1)).for_each(|dom| {
        let day = day_cycle.next().unwrap();

        if (day != &"saturday") && (day != &"sunday") {
            let empty_nurse_job = NursesJob { name: "".into(),
                                              for_trainees: Some(true) };
            let off_variant = (&"off".into(), &empty_nurse_job);
            let max_off_length = people_count -
                                 (cfg.rooms.len() * cfg.nurses_jobs.len());
            let off_iter = iter::repeat(off_variant).take(max_off_length);
            let mut job_variants = cfg.rooms
                                      .iter()
                                      .cartesian_product(&cfg.nurses_jobs)
                                      .chain(off_iter)
                                      .cycle()
                                      .skip(dom)
                                      .take(cfg.dates.end - cfg.dates.start)
                                      .collect::<Vec<(&String, &NursesJob)>>();

            let job_room_sep = cfg.job_room_sep.clone().unwrap_or(" ".into());

            cfg.people.nurses.iter().for_each(|n| {
                let its_vec = nurse_map.get_mut(&&n.name[..]).unwrap();

                if n.days.is_some() && !n.days
                                         .clone()
                                         .unwrap()
                                         .contains(&day.to_string()) {
                    (*its_vec).push("off (part time)".into());
                } else {
                    // Trainees need to mutate the jobs vector in a different
                    // way than front to back.
                    let next_pair = if n.trainee.unwrap_or(false) &&
                                       (job_variants.first()
                                                    .unwrap()
                                                    .0 != off_variant.0) {
                        let mut clone = job_variants.clone();
                        clone.retain(|j| j.1.for_trainees.unwrap_or(true));
                        let none_left_pop = "No trainee job variants left";
                        let none_left_find = "No trainee job variant left to \
                                              remove";
                        let variant = clone.pop()
                                           .expect(none_left_pop);
                        let idx = job_variants.iter()
                                              .position(|j| j.1
                                                             .for_trainees
                                                             .unwrap_or(true))
                                              .expect(none_left_find);
                        job_variants.remove(idx);
                        variant
                    } else {
                        job_variants.pop().expect("No job variants left")
                    };

                    (*its_vec).push(format!("{}{}{}",
                            next_pair.0,
                            &job_room_sep,
                            next_pair.1.name));
                }
            });
        }
    });

    for (n, jobs) in &nurse_map {
        let mut record = jobs.clone();
        record.insert(0, (*n).into());
        wtr.write_record(&record)?;
    }

    Ok(nurse_map.iter().next().unwrap().1.len())
}


fn write_empty(wtr: &mut csv::Writer<File>, size: usize) -> Result<()> {
    // Empty row
    let mut empty_record = vec![""; size];
    empty_record.insert(0, "");
    wtr.write_record(&empty_record)?;

    Ok(())
}


fn write_supporters(wtr: &mut csv::Writer<File>, cfg: &Config) -> Result<()> {
    // Supporters
    let mut supporter_map: HashMap<&str, Vec<String>> = HashMap::new();
    cfg.people.supporters.iter().for_each(|n| {
        supporter_map.insert(&n.name, vec![]);
    });

    let offset = VALID_WEEKDAYS.iter()
                               .position(|r| r == &&cfg.dates.start_day)
                               .unwrap();
    let mut day_cycle = VALID_WEEKDAYS.iter().cycle().skip(offset);

    let people_count = cfg.people.supporters.len();

    (cfg.dates.start..(cfg.dates.end + 1)).for_each(|dom| {
        let day = day_cycle.next().unwrap();

        if (day != &"saturday") && (day != &"sunday") {
            let off_variant = &"off".to_string();
            let off_iter = iter::repeat(off_variant).take(people_count -
                                                          cfg.rooms.len());
            let mut job_variants = cfg.rooms
                                      .iter()
                                      .chain(off_iter)
                                      .cycle()
                                      .skip(dom);

            cfg.people.supporters.iter().for_each(|n| {
                let its_vec = supporter_map.get_mut(&&n.name[..]).unwrap();

                if n.days.is_some() && !n.days
                                         .clone()
                                         .unwrap()
                                         .contains(&day.to_string()) {
                    (*its_vec).push("off-day".into());
                } else {
                    let next_job = job_variants.next().unwrap();
                    (*its_vec).push(format!("{}", next_job));
                }
            });
        }
    });

    for (n, jobs) in &supporter_map {
        let mut record = jobs.clone();
        record.insert(0, (*n).into());
        wtr.write_record(&record)?;
    }

    Ok(())
}


fn do_writes(mut wtr: &mut csv::Writer<File>, cfg: &Config) -> Result<()> {
    write_header(&mut wtr, &cfg.dates)?;

    let col_count = write_nurses(wtr, cfg)?;

    write_empty(wtr, col_count)?;

    write_supporters(wtr, cfg)?;

    Ok(())
}

fn run() -> Result<()> {
    let opt = Opt::parse();

    let input_err = || format!("Couldn't open {} as a file path.", &opt.input);
    let yaml_err = || format!("The input {} was an invalid yaml file.",
                              &opt.input);
    let out_err = || format!("Couldn't open {} for writing.", &opt.output);

    let f = OpenOptions::new().read(true)
                              .open(&opt.input)
                              .with_context(input_err)?;
    let cfg: Config = serde_yaml::from_reader(f).with_context(yaml_err)?;

    do_validates(&cfg)?;

    let mut out = OpenOptions::new().write(true)
                                    .create(true)
                                    .truncate(true)
                                    .open(&opt.output)
                                    .with_context(out_err)?;

    maybe_write_excel_sep(&mut out, &cfg)?;

    let mut wtr = csv::Writer::from_writer(out);

    do_writes(&mut wtr, &cfg)?;

    wtr.flush()?;

    println!("\n  File generated successfully into {}.", &opt.output);

    // Maybe await a keyboard event before closing.
    if !opt.block {
        println!("\nPress Enter when finished.");
        let _ = std::io::stdin().bytes().next().and_then(|r| r.ok());
    }

    Ok(())
}

fn main() -> Result<()> {
    run()
}
