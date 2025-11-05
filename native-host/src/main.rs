use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::process::Command;

const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum InMsg {
    Hello {
        #[allow(dead_code)]
        requested_version: Option<u16>
    },
    Ping { id: String },
    OpenDeepLink { url: String },
    OpenSession { id: String, read_only: bool },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OutMsg<'a> {
    Hello { version: u16 },
    Pong { id: &'a str },
    Ok { op: &'a str },
    Err { op: &'a str, message: String },
}

fn read_msg(stdin: &mut impl Read) -> Result<Option<serde_json::Value>> {
    let mut len_buf = [0u8; 4];
    if stdin.read_exact(&mut len_buf).is_err() { return Ok(None); }
    let len = u32::from_le_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stdin.read_exact(&mut buf).context("read payload")?;
    Ok(Some(serde_json::from_slice(&buf).context("json parse")?))
}

fn write_msg(stdout: &mut impl Write, v: &serde_json::Value) -> Result<()> {
    let bytes = serde_json::to_vec(v)?;
    stdout.write_all(&(bytes.len() as u32).to_le_bytes())?;
    stdout.write_all(&bytes)?;
    stdout.flush()?;
    Ok(())
}

fn open_url(url: &str) -> Result<()> {
    if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()?;
    } else if cfg!(target_os = "windows") {
        Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn()?;
    } else {
        Command::new("xdg-open").arg(url).spawn()?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    // Optional: send Hello immediately so the extension learns our version.
    write_msg(&mut stdout, &serde_json::to_value(OutMsg::Hello { version: PROTOCOL_VERSION })?)?;

    loop {
        let Some(v) = read_msg(&mut stdin)? else { break };
        let msg: Result<InMsg> = serde_json::from_value(v.clone()).context("invalid message");

        match msg {
            Ok(InMsg::Hello { requested_version: _ }) => {
                write_msg(&mut stdout, &serde_json::to_value(OutMsg::Hello { version: PROTOCOL_VERSION })?)?;
            }
            Ok(InMsg::Ping { id }) => {
                write_msg(&mut stdout, &serde_json::to_value(OutMsg::Pong { id: &id })?)?;
            }
            Ok(InMsg::OpenDeepLink { url }) => {
                let op = "open_deep_link";
                match open_url(&url) {
                    Ok(_) => write_msg(&mut stdout, &serde_json::to_value(OutMsg::Ok { op })?)?,
                    Err(e) => write_msg(&mut stdout, &serde_json::to_value(OutMsg::Err { op, message: e.to_string() })?)?,
                }
            }
            Ok(InMsg::OpenSession { id, read_only }) => {
                let op = "open_session";
                let url = format!("near://open/session/{}?readOnly={}", id, if read_only {1} else {0});
                match open_url(&url) {
                    Ok(_) => write_msg(&mut stdout, &serde_json::to_value(OutMsg::Ok { op })?)?,
                    Err(e) => write_msg(&mut stdout, &serde_json::to_value(OutMsg::Err { op, message: e.to_string() })?)?,
                }
            }
            Err(e) => {
                write_msg(&mut stdout, &serde_json::to_value(OutMsg::Err { op: "decode", message: e.to_string() })?)?;
            }
        }
    }
    Ok(())
}
