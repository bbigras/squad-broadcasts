use std::io::{BufRead, BufReader};
use std::fs::File;

use failure::{err_msg, Error, ResultExt};

use parsers::parse_map_broadcast;
use parsers::parse_map_names;
use parsers::MapName;

const MAPS_FILE: &str = "dpg-Maps.cfg";
const BROADCAST_FILE: &str = "dpg-Broadcasts.cfg";

fn load_map_names() -> Result<Vec<MapName>, Error> {
    let mut list = Vec::new();

    let f = BufReader::new(File::open(MAPS_FILE)?);

    for line in f.lines() {
        let parsed = parse_map_names(&line?)
            .to_full_result()
            .map_err(|e| format_err!("{:?}", e))
            .context("can't parse_map_names")?;
        list.push(parsed);
    }

    Ok(list)
}

pub fn get_broadcast(map_long_name: &str) -> Result<Option<String>, Error> {
    let maps = load_map_names()?;

    let map = maps.iter()
        .find(|m| map_long_name.starts_with(&m.long_name))
        .ok_or_else(|| err_msg("can't find map"))?;

    let f = File::open(BROADCAST_FILE)?;
    let f = BufReader::new(f);

    for line in f.lines() {
        let l = line?;

        let parsed = parse_map_broadcast(&l)
            .to_full_result()
            .map_err(|e| format_err!("{:?}", e))
            .context("can't parse_map_broadcast")?;
        if map.short_name == parsed.map {
            return Ok(Some(parsed.broadcast.to_string()));
        }
    }

    Ok(None)
}

#[test]
fn test_load_map_names() {
    let names = load_map_names().unwrap();
    assert_eq!(
        names[0],
        MapName {
            short_name: "Logar Valley AAS v1".to_string(),
            long_name: "/Game/Maps/Logar_Valley/LogarValley_AAS_v1".to_string(),
        }
    );
    assert_eq!(
        names[names.len() - 1],
        MapName {
            short_name: "Narva Invasion v1".to_string(),
            long_name: "/Game/Maps/Narva/Narva_Invasion_v1".to_string(),
        }
    )
}

#[test]
fn test_get_broadcast() {
    let r = get_broadcast("/Game/Maps/Logar_Valley/LogarValley_AAS_v1").unwrap();
    assert_eq!(
        r,
        Some("DO NOT RUSH POPPY/NORTH RESIDENCE OR MECHANIC/SOUTH RESIDENCE!".to_string())
    );

    let r = get_broadcast("/Game/Maps/Jensens_Range/Jensens_Range").unwrap();
    assert_eq!(r, None);
}
