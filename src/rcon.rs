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
        let mut data: &[u8] = include_bytes!("..\\tests\\ShowNextMap.bin");
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

    let data = rcon_gen(auth_id, &pw, SERVERDATA_AUTH)?;
    stream.take_error()?;

    stream.write_all(&data)?;
    stream.take_error()?;

    let resp1 = read_rcon_resp(&mut stream)?;
    stream.take_error()?;

    if resp1.id != auth_id || resp1.the_type != 0 || resp1.body.len() != 0 {
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
        let expect: &[u8] = include_bytes!("..\\tests\\cmd-ShowNextMap.bin");
        assert_eq!(result, expect);
    }

    {
        let result = rcon_gen(42, "pass123", SERVERDATA_AUTH).unwrap();
        let expect: &[u8] = include_bytes!("..\\tests\\auth-request.bin");
        assert_eq!(result, expect);
    }
}

pub fn exec<A: ToSocketAddrs>(addr: A, pw: &str, command: &str) -> Result<String> {
    let mut conn = connect(addr, pw)?;

    let cmd_bin = rcon_gen(1, command, SERVERDATA_EXECCOMMAND)?;
    conn.write_all(&cmd_bin)?;
    conn.take_error()?;

    Ok(read_rcon_resp(&mut conn)?.body)
}
