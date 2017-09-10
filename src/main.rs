#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate nom;

extern crate env_logger;
#[macro_use]
extern crate log;

extern crate stream_line_reader;

extern crate chrono;

extern crate toml;

#[macro_use]
extern crate serde_derive;

extern crate clap;

mod parsers;
mod maps;

extern crate rcon;

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

const BRINGING_WORLD: &'static [u8] = b"Bringing World";
const MATCH_STATE_CHANGED: &'static [u8] = b"Match State Changed from";
const CONFIG_FILE: &'static str = "broadcasts.toml";
const LOG_FILE: &'static str = "Squad.log";

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
            ParseInt(::std::num::ParseIntError);
        }

        links {
            Rcon(::rcon::errors::Error, ::rcon::errors::ErrorKind);
        }
    }
}

use errors::*;

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct Config {
    server: ServerConfig,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
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
                //let l = l.trim();

                if is_binging_world(l) {
                    let r = parse_bringing_world(l)
                        .to_full_result()
                        .map_err(|e| StringError(format!("{:?}", e)))
                        .chain_err(|| "can't parse_bringing_world")?;

                    if r.map != "/Game/Maps/TransitionMap.TransitionMap" {
                        if !is_preload {
                            info!("map> {}", r.map);
                        }

                        log_state.current_map = Some(r.map.to_string());
                    }
                } else if is_map_change(l) {
                    let r = parse_state_change(l)
                        .to_full_result()
                        .map_err(|e| StringError(format!("{:?}", e)))
                        .chain_err(|| "can't parse_state_change")?;
                    let parsed = parse_timestamp(r.timestamp)
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
                                        let cfg_clone = cfg.clone();
                                        thread::spawn(move || {
                                            info!("start loop");
                                            for sleep_time in &[10, 10, 30, 30] {
                                                thread::sleep(
                                                    time::Duration::from_secs(*sleep_time),
                                                );

                                                match rcon::exec(
                                                    (
                                                        cfg_clone.server.ip.as_str(),
                                                        cfg_clone.server.port as u16,
                                                    ),
                                                    &cfg_clone.server.pw,
                                                    &format!("AdminBroadcast {}", msg),
                                                ) {
                                                    Ok(resp) => info!("rcon response: {}", resp),
                                                    Err(e) => {
                                                        error!("error while broadcasting: {}", e)
                                                    }
                                                }
                                            }
                                            info!("loop ended");
                                        });
                                    }
                                } else {
                                    error!("current map is not set");
                                }
                            }
                        } else if r.to == "InProgress" && !is_preload {
                            info!("state> {} -> {}", r.from, r.to);

                            if let Some(ref map) = log_state.current_map {
                                if let Some(msg) = maps::get_broadcast(map)? {
                                    // send the broadcast twice
                                    for _ in 0..2 {
                                        match rcon::exec(
                                            (cfg.server.ip.as_str(), cfg.server.port as u16),
                                            &cfg.server.pw,
                                            &format!("AdminBroadcast {}", msg),
                                        ) {
                                            Ok(resp) => info!("rcon response: {}", resp),
                                            Err(e) => error!("error while broadcasting: {}", e),
                                        }
                                    }
                                }
                            } else {
                                error!("current map is not set");
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

fn load_config(file_name: &str) -> Result<Config> {
    let mut f = File::open(file_name)?;

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

    let cfg = load_config(CONFIG_FILE)?;

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

fn is_map_change(data: &[u8]) -> bool {
    if data.is_empty() {
        false
    } else if data.len() >= 68 && data[44..68] == *MATCH_STATE_CHANGED {
        true
    } else {
        data.len() >= 75 && data[51..75] == *MATCH_STATE_CHANGED
    }
}

#[test]
fn test_is_map_change() {
    let data1 = "[2017.07.28-02.47.55:163][312]LogGameState: Match State Changed from InProgress to WaitingPostMatch";
    let data2 = "[2017.07.28-02.48.23:803][639]LogGameMode:Display: Match State Changed from InProgress to WaitingPostMatch";
    let data3 = "[2017.07.26-17.11.04:357][569]LogRCONServer:Verbose: 7028:FRCONServer::Tick(): Message received:";
    let data4 = "";
    let data5 = "[2017.07.26-18.59.35:521][851]LogNet: Join succeeded: ВесёлыйКолбасниk";
    let data6 = "[2017.07.28-02.47.55:163][312]LogGameState: Match State Changed from";
    let data7 = "[2017.07.28-02.47.55:163][312]LogGameState: Match State Changed fro";
    let data8 = "[2017.07.28-02.48.23:803][639]LogGameMode:Display: Match State Changed from";
    let data9 = "[2017.07.28-02.48.23:803][639]LogGameMode:Display: Match State Changed fro";

    assert!(is_map_change(data1.as_bytes()));
    assert!(is_map_change(data2.as_bytes()));
    assert!(!is_map_change(data3.as_bytes()));
    assert!(!is_map_change(data4.as_bytes()));
    assert!(!is_map_change(data5.as_bytes()));
    assert!(is_map_change(data6.as_bytes()));
    assert!(!is_map_change(data7.as_bytes()));
    assert!(is_map_change(data8.as_bytes()));
    assert!(!is_map_change(data9.as_bytes()));
}

fn is_binging_world(data: &[u8]) -> bool {
    if data.is_empty() || data.len() < 54 {
        false
    } else {
        data.len() >= 54 && data[40..54] == *BRINGING_WORLD
    }
}

#[test]
fn test_is_binging_world() {
    let data1 = "[2017.07.27-22.30.32:563][814]LogWorld: Bringing World /Game/Maps/TransitionMap.TransitionMap up for play (max tick rate 50) at 2017.07.27-15.30.3";
    let data2 = "[2017.07.28-02.48.23:803][639]LogGameMode:Display: Match State Changed from InProgress to WaitingPostMatch";
    let data3 = "[2017.07.26-17.11.04:357][569]LogRCONServer:Verbose: 7028:FRCONServer::Tick(): Message received:";
    let data4 = "";
    let data5 = "[2017.07.26-18.59.35:521][851]LogNet: Join succeeded: ВесёлыйКолбасниk";
    let data6 = "[2017.07.27-22.30.32:563][814]LogWorld: Bringing World";
    let data7 = "[2017.07.27-22.30.32:563][814]LogWorld: Bringing Worl";

    assert!(is_binging_world(data1.as_bytes()));
    assert!(!is_binging_world(data2.as_bytes()));
    assert!(!is_binging_world(data3.as_bytes()));
    assert!(!is_binging_world(data4.as_bytes()));
    assert!(!is_binging_world(data5.as_bytes()));
    assert!(is_binging_world(data6.as_bytes()));
    assert!(!is_binging_world(data7.as_bytes()));
}

#[test]
fn test_load_config() {
    let cfg = load_config("tests/config.toml").unwrap();
    assert_eq!(
        cfg,
        Config {
            server: ServerConfig {
                ip: "ip".to_string(),
                port: 123456,
                pw: "rcon password".to_string(),
            },
        }
    );
}
