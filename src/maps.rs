use std::fs::File;
use std::io::{BufRead, BufReader};

use failure::{err_msg, Error, ResultExt};

use default_game::load_default_game_ini;
use parsers::parse_map_broadcast;

const BROADCAST_FILE: &str = "Broadcasts.cfg";

pub struct MapBroadcastOwned {
    pub map: String,
    pub broadcast: String,
}

pub fn load_broadcast_msg() -> Result<Vec<MapBroadcastOwned>, Error> {
    let f = File::open(BROADCAST_FILE).context(format!("can't open {}", BROADCAST_FILE))?;
    let f = BufReader::new(f);

    let mut list = Vec::new();

    for line in f.lines() {
        let l = line?;

        let parsed = parse_map_broadcast(&l)
            .to_full_result()
            .map_err(|e| format_err!("{:?}", e))
            .context("can't parse_map_broadcast")?;

        list.push(MapBroadcastOwned {
            map: parsed.map.to_string(),
            broadcast: parsed.broadcast.to_string(),
        });
    }

    Ok(list)
}

pub fn get_broadcast(map_long_name: &str) -> Result<Option<String>, Error> {
    let maps = load_default_game_ini()?;

    let map = maps
        .iter()
        .find(|m| map_long_name.starts_with(&m.long_name))
        .ok_or_else(|| err_msg("can't find map"))?;

    let f = File::open(BROADCAST_FILE).context(format!("can't open {}", BROADCAST_FILE))?;
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
fn test_get_broadcast() {
    let r = get_broadcast("/Game/Maps/Logar_Valley/LogarValley_AAS_v1").unwrap();
    assert_eq!(
        r,
        Some("DO NOT RUSH POPPY/NORTH RESIDENCE OR MECHANIC/SOUTH RESIDENCE!".to_string())
    );

    let r = get_broadcast("/Game/Maps/Jensens_Range/Jensens_Range").unwrap();
    assert_eq!(
        r,
        Some("WELCOME TO THE FIRING RANGE! FOLLOW ADMIN INSTRUCTIONS AND HAVE FUN!".to_string())
    );
}
