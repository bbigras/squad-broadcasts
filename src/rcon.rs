use std::str;
use std::net::{TcpStream, ToSocketAddrs};
use std::io::{Cursor, Read, Write};
use std::time::Duration;

use errors::*;
use StringError;

use nom::le_i32;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

const TIMEOUT_SECS: u64 = 5;

const SERVERDATA_AUTH_RESPONSE: i32 = 2;
const SERVERDATA_EXECCOMMAND: i32 = 2;
const SERVERDATA_AUTH: i32 = 3;
const SERVERDATA_RESPONSE_VALUE: i32 = 0;

#[derive(Debug, PartialEq)]
struct RconResponse {
    id: i32,
    the_type: i32,
    body: String, // TODO: test if Option<String> is lighter
}

named!(parse_rcon_response<&[u8], RconResponse>, do_parse!(
        id: le_i32 >>
        the_type: le_i32 >>
        body: map_res!(take_until_and_consume!("\0"), str::from_utf8) >>
        (RconResponse {
            id: id,
            the_type: the_type,
            body: body.into(),
        })
));

#[derive(Debug, PartialEq)]
struct RconResponseBin {
    id: i32,
    the_type: i32,
    body: Vec<u8>, // TODO: use &[u8] maybe
}

named!(parse_rcon_response_bin<&[u8], RconResponseBin>, do_parse!(
        id: le_i32 >>
        the_type: le_i32 >>
        body: take_until_and_consume!("\0") >>
        (RconResponseBin {
            id: id,
            the_type: the_type,
            body: body.into(),
        })
));

fn read_rcon_resp(stream: &mut Read) -> Result<RconResponse> {
    let mut buf1 = [0; 4];
    stream.read_exact(&mut buf1)?;

    let mut rdr = Cursor::new(buf1);
    let size = rdr.read_i32::<LittleEndian>()?;

    let mut buf2 = vec![0; size as usize];
    stream.read_exact(&mut buf2)?;

    let parsed = parse_rcon_response(&buf2)
        .to_full_result()
        .map_err(|e| StringError(format!("{:?}", e)))
        .chain_err(|| "can't parse_rcon_response")?;
    Ok(parsed)
}

fn read_rcon_resp_multi(stream: &mut Read, stop_id: i32) -> Result<(RconResponseBin, bool)> {
    let mut buf1 = [0; 4];
    stream.read_exact(&mut buf1)?;

    let mut rdr = Cursor::new(buf1);
    let size = rdr.read_i32::<LittleEndian>()?;

    let mut buf2 = vec![0; size as usize];
    stream.read_exact(&mut buf2)?;

    let parsed = parse_rcon_response_bin(&buf2)
        .to_full_result()
        .map_err(|e| StringError(format!("{:?}", e)))
        .chain_err(|| "can't parse_rcon_response")?;
    
    if parsed.id == stop_id {
        let mut buf3 = [0; 21];
        stream.read_exact(&mut buf3)?;

        let mut expect = vec![0x0a, 0x00, 0x00, 0x00];
        expect.write_i32::<LittleEndian>(stop_id)?;
        expect.append(&mut vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00]);
        
        if buf3 == expect.as_slice() {
            return Ok((parsed, true));
        }
    }

    Ok((parsed, false))
}

#[test]
fn test_read_rcon_resp() {
    {
        let data = vec![0x0a, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let reader = &mut data.as_slice();
        let rcon_resp = read_rcon_resp(reader).unwrap();

        assert_eq!(
            rcon_resp,
            RconResponse {
                id: 0,
                the_type: 0,
                body: "".to_string(),
            }
        );
    }

    {
        let data = vec![0x0a, 0, 0, 0, 0, 0, 0, 0, 0x02, 0, 0, 0, 0, 0];

        let reader = &mut data.as_slice();
        let rcon_resp = read_rcon_resp(reader).unwrap();

        assert_eq!(
            rcon_resp,
            RconResponse {
                id: 0,
                the_type: 2,
                body: "".to_string(),
            }
        );
    }

    {
        let mut data: &[u8] = include_bytes!("../tests/ShowNextMap.bin");
        let rcon_resp = read_rcon_resp(&mut data).unwrap();

        assert_eq!(
            rcon_resp,
            RconResponse {
                id: 1,
                the_type: 0,
                body: "Current map is Yehorivka AAS v1, Next map is Gorodok AAS v2".to_string(),
            }
        );
    }
}

fn rcon_gen(id: i32, data: &str, packet_type: i32) -> Result<Vec<u8>> {
    let mut wtr: Vec<u8> = Vec::new();
    wtr.write_i32::<LittleEndian>(id)?;
    wtr.write_i32::<LittleEndian>(packet_type)?;

    wtr.append(&mut data.into());
    wtr.append(&mut vec![0, 0]);

    let mut wtr2: Vec<u8> = Vec::new();
    wtr2.write_i32::<LittleEndian>(wtr.len() as i32)?;
    wtr2.append(&mut wtr);
    Ok(wtr2)
}

fn connect<A: ToSocketAddrs>(addr: A, pw: &str) -> Result<TcpStream> {
    let mut stream: TcpStream = TcpStream::connect(addr)?;
    stream
        .set_read_timeout(Some(Duration::new(TIMEOUT_SECS, 0)))?;
    stream
        .set_write_timeout(Some(Duration::new(TIMEOUT_SECS, 0)))?;

    let auth_id = 0;

    let data = rcon_gen(auth_id, pw, SERVERDATA_AUTH)?;
    stream.take_error()?;

    stream.write_all(&data)?;
    stream.flush()?;
    stream.take_error()?;

    let resp1 = read_rcon_resp(&mut stream)?;
    stream.take_error()?;

    if resp1.id != auth_id || resp1.the_type != 0 || !resp1.body.is_empty() {
        bail!("packet was supposed to be empty");
    }

    let resp2 = read_rcon_resp(&mut stream)?;
    stream.take_error()?;

    // TODO: test if the id is the same with a wrong password
    if resp2.id != auth_id || resp2.the_type != SERVERDATA_AUTH_RESPONSE {
        bail!("login failed");
    }

    Ok(stream)
}

#[test]
fn test_gen_exec_cmd() {
    {
        let result = rcon_gen(36, "ShowNextMap", SERVERDATA_EXECCOMMAND).unwrap();
        let expect: &[u8] = include_bytes!("../tests/cmd-ShowNextMap.bin");
        assert_eq!(result, expect);
    }

    {
        let result = rcon_gen(42, "pass123", SERVERDATA_AUTH).unwrap();
        let expect: &[u8] = include_bytes!("../tests/auth-request.bin");
        assert_eq!(result, expect);
    }
}

pub fn exec<A: ToSocketAddrs>(addr: A, pw: &str, command: &str) -> Result<String> {
    let mut conn = connect(addr, pw)?;

    let cmd_bin = rcon_gen(1, command, SERVERDATA_EXECCOMMAND)?;
    conn.write_all(&cmd_bin)?;
    conn.flush()?;
    conn.take_error()?;

    Ok(read_rcon_resp(&mut conn)?.body)
}

pub fn exec_big<A: ToSocketAddrs>(addr: A, pw: &str, command: &str) -> Result<String> {
    let mut conn = connect(addr, pw)?;

    let cmd_bin = rcon_gen(1, command, SERVERDATA_EXECCOMMAND)?;
    conn.write_all(&cmd_bin)?;
    conn.flush()?;
    conn.take_error()?;

    let empty_id = 2;

    let cmd_bin = rcon_gen(empty_id, "", SERVERDATA_RESPONSE_VALUE)?;
    conn.write_all(&cmd_bin)?;
    conn.flush()?;
    conn.take_error()?;

    let mut buf = Vec::new();

    loop {
        let (mut buf2, done) = read_rcon_resp_multi(&mut conn, empty_id).unwrap();
        buf.append(&mut buf2.body);

        if done {
            break;
        }
    }

    Ok(String::from_utf8_lossy(&buf).to_string())
}

#[test]
fn test_multi1() {
    let buf1 = vec![0x0C, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 97, 98, 0x00, 0x00];
    let buf2 = vec![0x0C, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 99, 100, 0x00, 0x00];
    let buf3 = vec![0x0a, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let buf4 = vec![0x0a, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00];

    let mut c = Cursor::new(buf1)
        .chain(Cursor::new(buf2))
        .chain(Cursor::new(buf3))
        .chain(Cursor::new(buf4));

    let mut buf = Vec::new();

    loop {
        let (mut buf2, done) = read_rcon_resp_multi(&mut c, 2).unwrap();
        buf.append(&mut buf2.body);

        if done {
            break;
        }
    }

    let decoded = String::from_utf8_lossy(&buf);

    assert_eq!(decoded, "abcd");
}

#[test]
fn test_multi2() {
    // test utf8 character splitted right in the middle (one byte on each packet, it should fail if we decode utf8 before merging both buffers)

    let buf1 = vec![0x0C, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 97, 195, 0x00, 0x00];
    let buf2 = vec![0x0C, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 169, 100, 0x00, 0x00];
    let buf3 = vec![0x0a, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let buf4 = vec![0x0a, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00];

    let mut c = Cursor::new(buf1)
        .chain(Cursor::new(buf2))
        .chain(Cursor::new(buf3))
        .chain(Cursor::new(buf4));

    let mut buf = Vec::new();

    loop {
        let (mut buf2, done) = read_rcon_resp_multi(&mut c, 2).unwrap();
        buf.append(&mut buf2.body);

        if done {
            break;
        }
    }

    let decoded = String::from_utf8_lossy(&buf);

    assert_eq!(decoded, "aéd");
}