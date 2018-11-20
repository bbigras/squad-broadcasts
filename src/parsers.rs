use nom::{digit, not_line_ending, rest};
use nom::types::{CompleteByteSlice, CompleteStr};

use std::str;

use nom_result;

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

    let parsed = parse_bringing_world(data.as_bytes()).map(nom_result).unwrap();

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

#[derive(Debug, PartialEq)]
pub struct StateChange2 {
    pub timestamp: String,
    pub from: String,
    pub to: String,
}

fn complete_byte_slice_to_str<'a>(s: CompleteByteSlice<'a>) -> Result<&'a str, str::Utf8Error> {
    str::from_utf8(s.0)
}

named!(pub parse_state_change<CompleteByteSlice, StateChange2>, ws!(do_parse!(
        tag!("[") >>
        timestamp: map_res!(take!(19), complete_byte_slice_to_str) >>
        tag!(":") >>
        digit >>
        tag!("][") >>
        digit >>
        take_until_and_consume!("Match State Changed from ") >>
        from: map_res!(take_until_and_consume!(" to "), complete_byte_slice_to_str) >>
        to: map_res!(rest, complete_byte_slice_to_str) >>

        (StateChange2 {
            timestamp: timestamp.to_string(),
            from: from.to_string(),
            to: to.to_string(),
        })
)));

#[test]
fn test_parse_state_change() {
    {
        let data = "[2017.02.19-07.46.23:777][999]LogGameMode:Display: Match State Changed from EnteringMap to WaitingToStart";

        let parsed = parse_state_change(CompleteByteSlice(data.as_bytes())).map(nom_result).unwrap();

        assert_eq!(parsed.timestamp, "2017.02.19-07.46.23"); // TODO: could be better
        assert_eq!(parsed.from, "EnteringMap");
        assert_eq!(parsed.to, "WaitingToStart");
    }

    {
        let data = "[2017.02.16-16.32.34:961][  0]LogGameState: Match State Changed from EnteringMap to WaitingToStart";

        let parsed = parse_state_change(CompleteByteSlice(data.as_bytes())).map(nom_result).unwrap();

        assert_eq!(parsed.timestamp, "2017.02.16-16.32.34"); // TODO: could be better
        assert_eq!(parsed.from, "EnteringMap");
        assert_eq!(parsed.to, "WaitingToStart");
    }
}

#[derive(Debug, PartialEq)]
pub struct Timestamp {
    pub year: String,
    pub month: String,
    pub day: String,
    pub hour: String,
    pub minute: String,
    pub second: String,
}

named!(pub parse_timestamp<CompleteStr, Timestamp>, ws!(do_parse!(
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
            year: year.to_string(),
            month: month.to_string(),
            day: day.to_string(),
            hour: hour.to_string(),
            minute: minute.to_string(),
            second: second.to_string(),
        })
)));

#[test]
fn test_parse_time() {
    let data = "2017.07.05-03.41.08";

    let parsed = parse_timestamp(CompleteStr(&data)).map(nom_result).unwrap();

    assert_eq!(parsed.year, "2017");
    assert_eq!(parsed.month, "07");
    assert_eq!(parsed.day, "05");
    assert_eq!(parsed.hour, "03");
    assert_eq!(parsed.minute, "41");
    assert_eq!(parsed.second, "08");
}

#[derive(Debug, PartialEq)]
pub struct MapBroadcast {
    pub map: String,
    pub broadcast: String,
}

named!(pub parse_map_broadcast<CompleteStr, MapBroadcast>, ws!(do_parse!(
        map: take_until_and_consume!("=") >>
        // broadcast: not_line_ending >>
        broadcast: rest >>

        (MapBroadcast {
            map: map.to_string(),
            broadcast: broadcast.to_string(),
        })
)));

#[test]
fn test_parse_map_broadcast() {
    let data =
        "/Game/Maps/BASRAH_CITY/Albasrah_aas_v1.Albasrah_aas_v1=DO NOT RUSH ANY VILLAGE/REFINERY";

    let parsed = parse_map_broadcast(CompleteStr(data)).map(nom_result).unwrap();

    assert_eq!(
        parsed.map,
        "/Game/Maps/BASRAH_CITY/Albasrah_aas_v1.Albasrah_aas_v1"
    );
    assert_eq!(parsed.broadcast, "DO NOT RUSH ANY VILLAGE/REFINERY");
}
