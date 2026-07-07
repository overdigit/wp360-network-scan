use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use rand::random;
use tokio::net::UdpSocket;

static HEADING: &str = "WP360 network scan tool";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Time between probes in milliseconds
    #[arg(short, long, default_value_t = 1000)]
    interval: u64,
    /// Amount of probes to send; endless by default
    #[arg(short, long, default_value_t = 0)]
    count: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("{}", "#".repeat(HEADING.len() + 4));
    println!("# {} #", " ".repeat(HEADING.len()));
    println!("# {HEADING} #");
    println!("# {} #", " ".repeat(HEADING.len()));
    println!("{}", "#".repeat(HEADING.len() + 4));
    println!("{}", "─".repeat(HEADING.len() * 2));
    let socket = UdpSocket::bind("0.0.0.0:8005").await?;
    socket.set_broadcast(true)?;

    let r = Arc::new(socket);
    let s = r.clone();

    let mut buf = vec![0; 255];
    let mut send_buf = Vec::new();
    send_buf.extend("WP360scan".bytes());

    let id = random::<u32>();

    send_buf.extend(id.to_le_bytes());

    let mut cache = HashSet::new();

    tokio::spawn(async move {
        let socket = s;
        let mut sent = 0;
        let mut interval = tokio::time::interval(Duration::from_millis(args.interval));
        while sent < args.count || args.count == 0 {
            match socket.send_to(&send_buf, "255.255.255.255:8005").await {
                Ok(_) => {
                    sent += 1;
                    interval.tick().await;
                }
                Err(e) => {
                    eprintln!("Sending scan failed: {e}");
                    break;
                }
            }
        }
    });

    let socket = r;
    loop {
        let (amt, src) = socket.recv_from(&mut buf).await?;
        if amt < 18 || !str::from_utf8(&buf[..9]).is_ok_and(|x| x == "WP360repl") {
            continue;
        }
        if buf[9] == 0 {
            println!("Device replied with an invalid packet.");
            continue;
        }
        let mut head = 9;

        let sender_id =
            u32::from_le_bytes([buf[head], buf[head + 1], buf[head + 2], buf[head + 3]]);
        if sender_id != id {
            // Not our packet
            continue;
        }
        head += 4;

        let reply_id = u32::from_le_bytes([buf[head], buf[head + 1], buf[head + 2], buf[head + 3]]);
        if !cache.insert(reply_id) {
            // Already received
            continue;
        }
        head += 4;

        let hostname_len = buf[head] as usize;
        if head + hostname_len > amt {
            eprintln!("Incomplete or malformed packet. [1]\n");
            continue;
        }
        head += 1;
        let hostname = str::from_utf8(&buf[head..head + hostname_len]).unwrap_or("N/A");
        println!("Device: {hostname} ({src})");
        head += hostname_len;
        while head < amt {
            let len = buf[head] as usize;
            head += 1;
            if head + len + 1 > amt {
                eprintln!("Incomplete or malformed packet. [2]\n");
                break;
            }

            let iname = str::from_utf8(&buf[head..head + len]).unwrap_or("N/A");
            head += len;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_precision_loss,
                clippy::cast_sign_loss,
                reason = "If any of these happen, we're gonna have bigger problems than indentation anyway"
            )]
            let tabs = (((iname.len() as f64) + 2.0) / 8.0 + 0.5) as usize;
            let addrs_n = buf[head] as usize;
            if head + addrs_n * 5 > amt {
                eprintln!("Incomplete or malformed packet. [3]\n");
                break;
            }
            head += 1;
            let mut indent = 1;
            print!("{iname}:");
            for _i in 0..addrs_n {
                println!(
                    "{}{}.{}.{}.{}/{}",
                    "\t".repeat(indent),
                    buf[head],
                    buf[head + 1],
                    buf[head + 2],
                    buf[head + 3],
                    buf[head + 4]
                );
                head += 5;
                indent = tabs;
            }
        }
        println!("{}", "─".repeat(HEADING.len() * 2));
    }
}
