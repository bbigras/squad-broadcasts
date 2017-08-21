use std::io::{BufRead, BufReader};
use std::fs::File;

use errors::*;
use StringError;

use parsers::parse_map_broadcast;
use parsers::parse_map_names;
use parsers::MapName;

static MAPS_FILE: &'static str = "dpg-Maps.cfg";
static BROADCAST_FILE: &'static str = "dpg-Broadcasts.cfg";

fn load_map_names() -> Result<Vec<MapName>> {
    let mut list = Vec::new();

    let f = BufReader::new(File::open(MAPS_FILE)?);

    for line in f.lines() {
        let parsed = parse_map_names(&line?)
            .to_full_result()
            .map_err(|e| StringError(format!("{:?}", e)))
            .chain_err(|| "can't parse_map_names")?;
        list.push(parsed);
    }

    Ok(list)
}

pub fn get_broadcast(map_long_name: &str) -> Result<Option<String>> {
    let maps = load_map_names()?;

    let map = maps.iter()
        .find(|m| map_long_name.starts_with(&m.long_name))
        .ok_or("can't find map".to_string())?;

    let f = File::open(BROADCAST_FILE)?;
    let f = BufReader::new(f);

    for line in f.lines() {
        let l = line?;

        let parsed = parse_map_broadcast(&l)
            .to_full_result()
            .map_err(|e| StringError(format!("{:?}", e)))
            .chain_err(|| "can't parse_map_broadcast")?;
        if map.short_name == parsed.map {
            return Ok(Some(parsed.broadcast.to_string()));
        }
    }

    Ok(None)
}
