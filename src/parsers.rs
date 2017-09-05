use nom::{digit, not_line_ending};
use std::str;

#[derive(Debug, PartialEq)]
pub struct Map<'a> {
    pub timestamp: &'a str,
    pub map: &'a str,
}

named!(pub parse_bringing_world<&[u8], Map>, ws!(do_parse!(
        tag!("[") >>
        timestamp: map_res!(take!(19), str::from_utf8) >>
        tag!(":") >>
        digit >>
        tag!("][") >>
        digit >>
        take_until_and_consume!("Bringing World ") >>
        map:  map_res!(take_until!(" up for play"), str::from_utf8) >>
        (Map {
            timestamp: timestamp,
            map:map,
        })
)));

#[test]
fn test_parse_bringing_world() {
    let data = "[2017.02.16-16.32.34:844][  0]LogWorld: Bringing World /Game/Maps/Sumari/Sumari_aas_v3/Sumari_aas_v3.Sumari_AAS_v3 up for play (max tick rate 60) at 2017.02.16-16.32.34";

    let parsed = parse_bringing_world(data.as_bytes()).to_result().unwrap();

    assert_eq!(parsed.timestamp, "2017.02.16-16.32.34");
    assert_eq!(
        parsed.map,
        "/Game/Maps/Sumari/Sumari_aas_v3/Sumari_aas_v3.Sumari_AAS_v3"
    );
}

#[derive(Debug, PartialEq)]
pub struct StateChange<'a> {
    pub timestamp: &'a str,
    pub from: &'a str,
    pub to: &'a str,
}

named!(pub parse_state_change<&[u8], StateChange>, ws!(do_parse!(
        tag!("[") >>
        timestamp: map_res!(take!(19), str::from_utf8) >>
        tag!(":") >>
        digit >>
        tag!("][") >>
        digit >>
        take_until_and_consume!("Match State Changed from ") >>
        from: map_res!(take_until_and_consume!(" to "), str::from_utf8) >>
        to: map_res!(not_line_ending, str::from_utf8) >>
       
        (StateChange {
            timestamp: timestamp.into(),
            from: from.into(),
            to: to.into(),
        })
)));

#[test]
fn test_parse_state_change() {
    {
        let data = "[2017.02.19-07.46.23:777][999]LogGameMode:Display: Match State Changed from EnteringMap to WaitingToStart";

        let parsed = parse_state_change(data.as_bytes()).to_result().unwrap();

        assert_eq!(parsed.timestamp, "2017.02.19-07.46.23"); // TODO: could be better
        assert_eq!(parsed.from, "EnteringMap");
        assert_eq!(parsed.to, "WaitingToStart");
    }

    {
        let data = "[2017.02.16-16.32.34:961][  0]LogGameState: Match State Changed from EnteringMap to WaitingToStart";

        let parsed = parse_state_change(data.as_bytes()).to_result().unwrap();

        assert_eq!(parsed.timestamp, "2017.02.16-16.32.34"); // TODO: could be better
        assert_eq!(parsed.from, "EnteringMap");
        assert_eq!(parsed.to, "WaitingToStart");
    }
}

#[derive(Debug, PartialEq)]
pub struct Timestamp<'a> {
    pub year: &'a str,
    pub month: &'a str,
    pub day: &'a str,
    pub hour: &'a str,
    pub minute: &'a str,
    pub second: &'a str,
}

named!(pub parse_timestamp<&str, Timestamp>, ws!(do_parse!(
        year: take!(4) >>
        tag!(".") >>
        month: take!(2) >>
        tag!(".") >>
        day: take!(2) >>
        tag!("-") >>
        hour: take!(2) >>
        tag!(".") >>
        minute: take!(2) >>
        tag!(".") >>
        second: take!(2) >>
        
        (Timestamp {
            year: year,
            month: month,
            day: day,
            hour: hour,
            minute: minute,
            second: second,
        })
)));

#[test]
fn test_parse_time() {
    let data = "2017.07.05-03.41.08";

    let parsed = parse_timestamp(&data).to_result().unwrap();

    assert_eq!(parsed.year, "2017");
    assert_eq!(parsed.month, "07");
    assert_eq!(parsed.day, "05");
    assert_eq!(parsed.hour, "03");
    assert_eq!(parsed.minute, "41");
    assert_eq!(parsed.second, "08");
}

#[derive(Debug, PartialEq)]
pub struct MapName {
    pub short_name: String,
    pub long_name: String,
}

named!(pub parse_map_names<&str, MapName>, ws!(do_parse!(
        short_name: take_until_and_consume!("=") >>
        long_name: not_line_ending >>
        
        (MapName {
            short_name: short_name.into(),
            long_name: long_name.into(),
        })
)));

#[test]
fn test_parse_map_names() {
    let data = "Logar PAAS V1=/Game/Maps/Logar_Valley/Logar_Valley_PAAS/LogarValley_PAAS_v1";

    let parsed = parse_map_names(&data).to_result().unwrap();

    assert_eq!(parsed.short_name, "Logar PAAS V1");
    assert_eq!(
        parsed.long_name,
        "/Game/Maps/Logar_Valley/Logar_Valley_PAAS/LogarValley_PAAS_v1"
    );
}

#[derive(Debug, PartialEq)]
pub struct MapBroadcast<'a> {
    pub map: &'a str,
    pub broadcast: &'a str,
}

named!(pub parse_map_broadcast<&str, MapBroadcast>, ws!(do_parse!(
        map: take_until_and_consume!("=") >>
        broadcast: not_line_ending >>
        
        (MapBroadcast {
            map: map.into(),
            broadcast: broadcast.into(),
        })
)));

#[test]
fn test_parse_map_broadcast() {
    let data = "/Game/Maps/BASRAH_CITY/Albasrah_aas_v1.Albasrah_aas_v1=DO NOT RUSH ANY VILLAGE/REFINERY";

    let parsed = parse_map_broadcast(&data).to_result().unwrap();

    assert_eq!(
        parsed.map,
        "/Game/Maps/BASRAH_CITY/Albasrah_aas_v1.Albasrah_aas_v1"
    );
    assert_eq!(parsed.broadcast, "DO NOT RUSH ANY VILLAGE/REFINERY");
}
