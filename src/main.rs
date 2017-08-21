#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate nom;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate stream_line_reader;

extern crate chrono;

extern crate byteorder;

extern crate toml;
extern crate serde;

#[macro_use]
extern crate serde_derive;

extern crate clap;

mod parsers;
mod maps;
mod rcon;

use parsers::parse_bringing_world;
use parsers::parse_state_change;
use parsers::parse_timestamp;

use stream_line_reader::StreamReader;

use log::{LogLevelFilter, LogRecord};
use env_logger::LogBuilder;

use chrono::offset::{Local, Utc};
use chrono::{DateTime, TimeZone};

use clap::{App, Arg};

use std::io::Read;
use std::fs::{metadata, File};
use std::env;
use std::{error, fmt};
use std::{thread, time};

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            ParseInt(::std::num::ParseIntError);
        }
    }
}

use errors::*;

static LOG_FILE: &'static str = "Squad.log";

#[derive(Debug, Deserialize)]
struct Config {
    server: ServerConfig,
}

#[derive(Debug, Deserialize)]
struct ServerConfig {
    ip: String,
    port: u64,
    pw: String,
}

// TODO: shouldn't need this
#[derive(Debug)]
pub struct StringError(String);

impl fmt::Display for StringError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&self.0)
    }
}

impl error::Error for StringError {
    fn description(&self) -> &str {
        &self.0
    }
}

struct StateTime {
    state: String,
    datetime: DateTime<chrono::Utc>,
}

struct LogState {
    current_map: Option<String>,
    last_state_change: Option<StateTime>,
    last_file_size: Option<u64>,
}

fn follow_log<R: Read>(
    reader: &mut StreamReader<R>,
    log_state: &mut LogState,
    cfg: &Config,
) -> Result<()> {
    let mut is_preload = true;

    info!("preloading log");

    loop {
        match reader.line() {
            Ok(l) => {
                if l.is_none() {
                    if is_preload {
                        info!("preloading done");
                    }
                    //TODO: would be better to check for EOF

                    is_preload = false;

                    // Check if the log file rotated
                    let metadata = metadata(LOG_FILE)?;

                    if let Some(l) = log_state.last_file_size {
                        if metadata.len() < l {
                            info!("file is smaller, reopen");
                            return Ok(());
                        }
                    }
                    log_state.last_file_size = Some(metadata.len());

                    debug!("sleep");
                    thread::sleep(time::Duration::from_secs(1));

                    continue;
                }

                let l = l.unwrap();
                let l = l.trim();

                if l.contains("Bringing World") && !l.contains("TransitionMap") {
                    let r = parse_bringing_world(&l)
                        .to_full_result()
                        .map_err(|e| StringError(format!("{:?}", e)))
                        .chain_err(|| "can't parse_bringing_world")?;

                    if !is_preload {
                        info!("map> {}", r.map);
                    }

                    log_state.current_map = Some(r.map.to_string());
                } else if l.contains("Match State Changed from") {
                    let r = parse_state_change(&l)
                        .to_full_result()
                        .map_err(|e| StringError(format!("{:?}", e)))
                        .chain_err(|| "can't parse_state_change")?;
                    let parsed = parse_timestamp(&r.timestamp)
                        .to_full_result()
                        .map_err(|e| StringError(format!("{:?}", e)))
                        .chain_err(|| "can't parse_timestamp")?;

                    let datetime = Utc.ymd(
                        parsed.year.parse()?,
                        parsed.month.parse()?,
                        parsed.day.parse()?,
                    ).and_hms(
                            parsed.hour.parse()?,
                            parsed.minute.parse()?,
                            parsed.second.parse()?,
                        );

                    let ignore_change = {
                        match log_state.last_state_change {
                            Some(ref t) => {
                                t.state == r.to &&
                                    datetime.signed_duration_since(t.datetime).num_seconds() < 5
                            }
                            None => false,
                        }
                    };

                    if !ignore_change {
                        if r.to == "WaitingToStart" {
                            if !is_preload {
                                info!("state> {} -> {}", r.from, r.to);

                                if let Some(ref map) = log_state.current_map {
                                    if let Some(msg) = maps::get_broadcast(map)? {
                                        thread::spawn(move || {
                                            info!("start loop");
                                            for sleep_time in vec![10, 10, 30, 30] {
                                                thread::sleep(
                                                    time::Duration::from_secs(sleep_time),
                                                );
                                                info!("would broadcast: {}\n", msg);
                                            }
                                            info!("loop ended");
                                        });
                                    }
                                } else {
                                    error!("current map is not set");
                                }
                            }
                        } else if r.to == "InProgress" {
                            if !is_preload {
                                info!("state> {} -> {}", r.from, r.to);

                                if let Some(ref map) = log_state.current_map {
                                    if let Some(msg) = maps::get_broadcast(map)? {
                                        info!("would broadcast: {}\n", msg);
                                    }
                                } else {
                                    error!("current map is not set");
                                }
                            }
                        }
                    }

                    log_state.last_state_change = Some(StateTime {
                        state: r.to.to_string(),
                        datetime: datetime,
                    });
                }
            }
            Err(e) => {
                println!("error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

fn init_log() {
    let format = |record: &LogRecord| {
        format!(
            "{} - {} - {}",
            Local::now().format("%Y-%m-%d %H:%M:%S,%f"),
            record.level(),
            record.args()
        )
    };

    let mut builder = LogBuilder::new();
    builder
        .format(format)
        .filter(None, LogLevelFilter::Info)
        .filter(Some("reqwest"), LogLevelFilter::Warn);

    if env::var("RUST_LOG").is_ok() {
        builder.parse(&env::var("RUST_LOG").unwrap());
    }

    builder.init().unwrap();
}

fn open_log(cfg: &Config) -> Result<()> {
    let f = File::open(LOG_FILE)?;

    let mut reader = StreamReader::new(f);

    let mut log_state = LogState {
        current_map: None,
        last_state_change: None,
        last_file_size: None,
    };

    follow_log(&mut reader, &mut log_state, cfg)?;

    Ok(())
}

fn load_config() -> Result<Config> {
    let mut f = File::open("broadcasts.toml")?;

    let mut buffer = String::new();
    f.read_to_string(&mut buffer)?;

    let cfg: Config = toml::from_str(&buffer).unwrap();
    Ok(cfg)
}

fn run() -> Result<()> {
    init_log();

    let matches = App::new("squad auto broadcasts")
        .arg(Arg::with_name("test").long("test").help("test rcon"))
        .get_matches();

    let cfg = load_config()?;

    if matches.is_present("test") {
        info!("test rcon");
        info!("running the ShowNextMap command");
        let next_map = rcon::exec(
            (cfg.server.ip.as_str(), cfg.server.port as u16),
            &cfg.server.pw,
            "ShowNextMap",
        )?;
        println!("result: {}", next_map);
        return Ok(());
    }

    info!("launch");

    loop {
        open_log(&cfg)?;
        thread::sleep(time::Duration::from_secs(2));
    }
}

quick_main!(run);
