use std::fs::File;
use std::io::{BufRead, BufReader};

use failure::{Error, ResultExt};

use nom_result;
use nom_err;

pub fn load_default_game_ini() -> Result<Vec<MapShortLong>, Error> {
    let f = File::open("DefaultGame.ini").context("can't open DefaultGame.ini")?;
    let reader = BufReader::new(f);

    let mut list = Vec::new();

    for line in reader.lines() {
        let l = line.unwrap();

        if l.starts_with("+ValidMapsList=") {
            let parsed = parse_map_line(&l).map(nom_result)
                .map_err(nom_err)?;

            if parsed.paths.len() == 1 {
                list.push(MapShortLong {
                    short_name: parsed.short_name.to_string(),
                    long_name: parsed.paths.iter().next().unwrap().to_string(), // TODO .first()
                });
            }
        }
    }

    Ok(list)
}

#[derive(Debug, PartialEq)]
struct Map<'a> {
    short_name: &'a str,
    paths: Vec<&'a str>,
}

named!(parse_map_path<&str, &str>, ws!(do_parse!(
    tag!("\"") >>
        path: take_until_and_consume!("\"") >>
        opt!(tag!(", ")) >>
        (path)
)));

named!(parse_map_line<&str, Map>, ws!(do_parse!(
    tag!(r#"+ValidMapsList=(ShortName=""#) >>
        short_name: take_until_and_consume!("\", MapPaths=(") >>
        paths: many1!(parse_map_path) >>
        (Map{
            short_name: short_name.into(),
            paths: paths,
        })
)));

#[test]
fn test() {
    let data = r#"+ValidMapsList=(ShortName="Logar", MapPaths=("/Game/Maps/Logar_Valley/LogarValley_AAS_v1", "/Game/Maps/Logar_Valley/Logar_Valley_2/LogarValley_AAS_INF_v1", "/Game/Maps/Logar_Valley/Logar_Valley_2_INS/LogarValley_INS_v1", "/Game/Maps/Logar_Valley/Logar_Valley_3_INS/LogarValley_INS_v1_Night", "/Game/Maps/Logar_Valley/Logar_Valley_PAAS/LogarValley_PAAS_v1"), LoadingScreenTexturePath="/Game/UI/Menu/LoadingScreen.LoadingScreen")"#;
    let parsed = parse_map_line(&data).map(nom_result).unwrap();
    assert_eq!(parsed.short_name, "Logar".to_string());
    assert_eq!(
        parsed.paths,
        vec![
            "/Game/Maps/Logar_Valley/LogarValley_AAS_v1",
            "/Game/Maps/Logar_Valley/Logar_Valley_2/LogarValley_AAS_INF_v1",
            "/Game/Maps/Logar_Valley/Logar_Valley_2_INS/LogarValley_INS_v1",
            "/Game/Maps/Logar_Valley/Logar_Valley_3_INS/LogarValley_INS_v1_Night",
            "/Game/Maps/Logar_Valley/Logar_Valley_PAAS/LogarValley_PAAS_v1"
        ]
    );
}

#[test]
fn test2() {
    let data = r#"+ValidMapsList=(ShortName="Fool's Road AAS v1", MapPaths=("/Game/Maps/Fools_Road/FoolsRoad_AAS_v1"), LoadingScreenTexturePath="/Game/UI/Menu/LoadingScreen.LoadingScreen")"#;
    let parsed = parse_map_line(&data).map(nom_result).unwrap();
    assert_eq!(parsed.short_name, "Fool's Road AAS v1");
    assert_eq!(parsed.paths, vec!["/Game/Maps/Fools_Road/FoolsRoad_AAS_v1"]);
}

#[derive(Debug)]
pub struct MapShortLong {
    pub short_name: String,
    pub long_name: String,
}
