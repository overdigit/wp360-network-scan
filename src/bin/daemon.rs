use std::collections::HashMap;
use std::net::UdpSocket;

use gethostname::gethostname;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind("0.0.0.0:8005")?;
    socket.set_broadcast(true)?;
    let mut buf = vec![0; 255];

    // TODO: init cache store with req.id as key, save first and last request timestamp,
    //       generate response ID and reuse it if req.id is in cache

    let mut cache = HashMap::new();

    loop {
        let (amt, _src) = socket.recv_from(&mut buf)?;
        if amt < 13 || !str::from_utf8(&buf[..9]).is_ok_and(|x| x == "WP360scan") {
            continue;
        }

        let id = u32::from_le_bytes([buf[9], buf[10], buf[11], buf[12]]);
        let res_id = cache.entry(id).or_insert_with(rand::random::<u32>);

        let ifaces = get_if_addrs::get_if_addrs()?;
        let addrs = ifaces
            .iter()
            .filter_map(|i| match &i.addr {
                get_if_addrs::IfAddr::V4(a) if !a.ip.is_loopback() => {
                    Some((&i.name, a.ip, a.netmask))
                }
                _ => None,
            })
            .fold(HashMap::new(), |mut acc, (iname, ip, netmask)| {
                acc.entry(iname)
                    .or_insert_with(Vec::new)
                    .push((ip, netmask));
                acc
            });

        buf.clear();
        buf.extend("WP360repl".bytes());
        buf.extend(id.to_le_bytes());
        buf.extend(res_id.to_le_bytes());

        let hostname_raw = gethostname();

        let hostname = match hostname_raw.to_str() {
            Some(s) if s.len() <= 255 => s,
            _ => "N/A",
        };

        buf.push(hostname.len().try_into().unwrap());
        buf.extend(hostname.bytes());

        let start = buf.len();

        for (iname, ips) in &addrs {
            if iname.len() > 255 || ips.len() > 255 {
                buf[start] = 0;
                break;
            }
            buf.push(iname.len().try_into().unwrap());
            buf.extend(iname.bytes());
            buf.push(ips.len().try_into().unwrap());
            for (a, nm) in ips {
                buf.extend(a.octets());
                buf.push(nm.to_bits().leading_ones().try_into().unwrap());
            }
        }
        socket.send_to(&buf, "255.255.255.255:8005")?;
    }
    //Ok(())
}
