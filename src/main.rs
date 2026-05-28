use std::env;
use std::fs::File;
use std::io::{self, Cursor, BufReader, BufRead, IsTerminal};
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::f64;

const MAX_RESPONSE_SIZE: usize = 512;
const CONCURRENCY_LIMIT: usize = 16;
const DEFAULT_DNS_SERVERS: [&str; 42] = [
    // Alibaba
    "223.5.5.5:53",
    "223.6.6.6:53",
    // Cloudflare
    "1.1.1.1:53",
    "1.0.0.1:53",
    "1.1.1.2:53",
    "1.0.0.2:53",
    "1.1.1.3:53",
    "1.0.0.3:53",
    // Google
    "8.8.8.8:53",
    "8.8.4.4:53",
    // Gcore
    "95.85.95.85:53",
    "2.56.220.2:53",
    // Vercara
    "156.154.70.1:53",
    "156.154.71.1:53",
    "156.154.70.2:53",
    "156.154.71.2:53",
    "156.154.70.3:53",
    "156.154.71.3:53",
    "156.154.70.4:53",
    "156.154.71.4:53",
    "156.154.70.5:53",
    "156.154.71.5:53",
    // OpenDNS
    "208.67.222.222:53",
    "208.67.220.220:53",
    "208.67.222.123:53",
    "208.67.220.123:53",
    "208.67.222.2:53",
    "208.67.220.2:53",
    // Oracle
    "216.146.35.35:53",
    "216.146.36.36:53",
    // Quad9
    "9.9.9.9:53",
    "149.112.112.112:53",
    "9.9.9.11:53",
    "149.112.112.11:53",
    "9.9.9.10:53",
    "149.112.112.10:53",
    // Yandex
    "77.88.8.8:53",
    "77.88.8.1:53",
    "77.88.8.88:53",
    "77.88.8.2:53",
    "77.88.8.7:53",
    "77.88.8.3:53"
];

#[derive(Copy, Clone)]
enum QueryType {
    A,
    Aaaa,
    Txt,
    Ns,
}

impl QueryType {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "A"    => Some(Self::A),
            "AAAA" => Some(Self::Aaaa),
            "TXT"  => Some(Self::Txt),
            "NS"   => Some(Self::Ns),
            _      => None,
        }
    }

    fn type_code(self) -> [u8; 2] {
        match self {
            Self::A    => [0x00, 0x01],
            Self::Aaaa => [0x00, 0x1C],
            Self::Txt  => [0x00, 0x10],
            Self::Ns   => [0x00, 0x02],
        }
    }
}

#[derive(Copy, Clone)]
enum SortBy { Min, Avg, Max, StdDev, Loss }

impl SortBy {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "min"    => Some(Self::Min),
            "avg"    => Some(Self::Avg),
            "max"    => Some(Self::Max),
            "stddev" => Some(Self::StdDev),
            "loss"   => Some(Self::Loss),
            _        => None,
        }
    }
}

#[derive(Copy, Clone)]
enum OutputFormat { Text, Json }

impl OutputFormat {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            _      => None,
        }
    }
}

struct ResultRow {
    server:   String,
    min:      f64,
    avg:      f64,
    max:      f64,
    stddev:   f64,
    loss:     f64,
    is_error: bool,
}

impl ResultRow {
    fn sort_key(&self, by: SortBy) -> f64 {
        if self.is_error || self.loss >= 100.0 {
            return f64::MAX;
        }
        match by {
            SortBy::Min    => self.min,
            SortBy::Avg    => self.avg,
            SortBy::Max    => self.max,
            SortBy::StdDev => self.stddev,
            SortBy::Loss   => self.loss,
        }
    }

    fn to_text(&self) -> String {
        let name = display_server(&self.server);
        if self.is_error {
            format!("{:<25} error", name)
        } else if self.loss >= 100.0 {
            format!("{:<25} {:<10} {:<10} {:<10} {:<12} {:<10.1}", name, "---", "---", "---", "---", self.loss)
        } else {
            format!("{:<25} {:<10.3} {:<10.3} {:<10.3} {:<12.3} {:<10.1}",
                name, self.min, self.avg, self.max, self.stddev, self.loss)
        }
    }

    fn to_json(&self) -> String {
        let name = display_server(&self.server);
        if self.is_error {
            format!(r#"{{"server":"{}","error":true}}"#, name)
        } else {
            format!(
                r#"{{"server":"{}","min":{:.3},"avg":{:.3},"max":{:.3},"stddev":{:.3},"loss":{:.1}}}"#,
                name, self.min, self.avg, self.max, self.stddev, self.loss
            )
        }
    }
}

fn flag_val<'a>(args: &'a [String], long: &str, short: &str) -> Option<&'a str> {
    let long_key = format!("--{}", long);
    let short_key = format!("-{}", short);
    args.windows(2)
        .find(|w| w[0] == long_key || w[0] == short_key)
        .map(|w| w[1].as_str())
}

fn flag_exists(args: &[String], long: &str, short: &str) -> bool {
    let long_key = format!("--{}", long);
    let short_key = format!("-{}", short);
    args.iter().any(|a| a == &long_key || a == &short_key)
}

fn require_flag(args: &[String], long: &str, short: &str) -> String {
    flag_val(args, long, short).unwrap_or_else(|| {
        eprintln!("--{} is required", long);
        std::process::exit(1);
    }).to_string()
}

fn parse_qtype(s: &str) -> QueryType {
    QueryType::from_str(s).unwrap_or_else(|| {
        eprintln!("Unknown query type '{}'. Supported: A, AAAA, TXT, NS", s);
        std::process::exit(1);
    })
}

fn parse_sort_by(s: &str) -> SortBy {
    SortBy::from_str(s).unwrap_or_else(|| {
        eprintln!("Unknown sort field '{}'. Supported: min, avg, max, stddev, loss", s);
        std::process::exit(1);
    })
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage_and_exit();
    }

    if args[1] == "--version" || args[1] == "-V" {
        println!("dnstracer {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let subcommand = args[1].as_str();
    let flags = &args[2..];

    match subcommand {
        "ping" => {
            let domain = require_flag(flags, "domain", "d");
            let server = normalize_server(&require_flag(flags, "server", "s"));
            let interval = flag_val(flags, "interval", "i").unwrap_or("1")
                .parse::<u64>().unwrap_or_else(|_| { eprintln!("--interval must be a number"); std::process::exit(1); });
            let count = flag_val(flags, "count", "c").unwrap_or("5")
                .parse::<u32>().unwrap_or_else(|_| { eprintln!("--count must be a number"); std::process::exit(1); });
            let timeout = Duration::from_secs(flag_val(flags, "timeout", "T").unwrap_or("5")
                .parse::<u64>().unwrap_or_else(|_| { eprintln!("--timeout must be a number"); std::process::exit(1); }));
            let show_plot = flag_exists(flags, "plot", "p");
            let qtype = parse_qtype(flag_val(flags, "type", "t").unwrap_or("A"));
            ping(&domain, &server, interval, count, show_plot, qtype, timeout)?;
        }
        "compare" => {
            let domain = require_flag(flags, "domain", "d");
            let dns_file = flag_val(flags, "file", "f").unwrap_or("").to_string();
            let interval = flag_val(flags, "interval", "i").unwrap_or("1")
                .parse::<u64>().unwrap_or_else(|_| { eprintln!("--interval must be a number"); std::process::exit(1); });
            let count = flag_val(flags, "count", "c").unwrap_or("5")
                .parse::<u32>().unwrap_or_else(|_| { eprintln!("--count must be a number"); std::process::exit(1); });
            let timeout = Duration::from_secs(flag_val(flags, "timeout", "T").unwrap_or("5")
                .parse::<u64>().unwrap_or_else(|_| { eprintln!("--timeout must be a number"); std::process::exit(1); }));
            let qtype = parse_qtype(flag_val(flags, "type", "t").unwrap_or("A"));
            let sort_by = parse_sort_by(flag_val(flags, "sort", "S").unwrap_or("avg"));
            let out_fmt = OutputFormat::from_str(flag_val(flags, "output", "o").unwrap_or("text"))
                .unwrap_or_else(|| { eprintln!("--output must be text or json"); std::process::exit(1); });
            let watch_secs = flag_val(flags, "watch", "w").map(|s| s.parse::<u64>()
                .unwrap_or_else(|_| { eprintln!("--watch must be a number"); std::process::exit(1); }))
                .unwrap_or(0);
            compare(&domain, &dns_file, interval, count, qtype, sort_by, timeout, out_fmt, watch_secs)?;
        }
        _ => {
            print_usage_and_exit();
        }
    }

    Ok(())
}

fn print_usage_and_exit() {
    println!( 
    r#"
   ____              _____                        
  |  _ \ _ __  ___  |_   _| __ __ _  ___ ___ _ __ 
  | | | | '_ \/ __|   | || '__/ _` |/ __/ _ \ '__|
  | |_| | | | \__ \   | || | | (_| | (_|  __/ |   
  |____/|_| |_|___/   |_||_|  \__,_|\___\___|_|                                                  

   DNS Tracer Tool v{}

   A tool to measure and analyze DNS query response times for network performance and latency.

   Developed by: @milad_bahari
 
   USAGE:
     dnstracer ping -d <domain> -s <server> [-i <seconds>] [-c <n>] [-T <seconds>] [-p] [-t A|AAAA|TXT|NS]
       - -d, --domain:    The domain name to query.
       - -s, --server:    DNS server — IPv4 (1.1.1.1), IPv6 (2606:4700::1111), or with port (1.1.1.1:5353).
       - -i, --interval:  Time in seconds between each query (default: 1).
       - -c, --count:     Number of queries to perform (default: 5).
       - -T, --timeout:   Query timeout in seconds (default: 5).
       - -p, --plot:      Display a plot of response times.
       - -t, --type:      Query type (default: A). Supported: A, AAAA, TXT, NS.

     dnstracer compare -d <domain> [-f <dns_file>] [-i <seconds>] [-c <n>] [-T <seconds>] [-t A|AAAA|TXT|NS] [-S min|avg|max|stddev|loss] [-o text|json] [-w <seconds>]
       - -d, --domain:    The domain name to query.
       - -f, --file:      Path to a file containing DNS servers (uses built-in list if omitted).
       - -i, --interval:  Time in seconds between each query (default: 1).
       - -c, --count:     Number of queries to perform (default: 5).
       - -T, --timeout:   Query timeout in seconds (default: 5).
       - -t, --type:      Query type (default: A). Supported: A, AAAA, TXT, NS.
       - -S, --sort:      Sort results by field (default: avg). Supported: min, avg, max, stddev, loss.
       - -o, --output:    Output format (default: text). Supported: text, json.
       - -w, --watch:     Re-run every N seconds and refresh the screen.

   EXAMPLES:
     dnstracer --version
     dnstracer ping -d google.com -s 1.1.1.1 -i 5 -c 10 -p
     dnstracer ping -d google.com -s 2606:4700:4700::1111 -c 10 -t AAAA
     dnstracer compare -d google.com -i 5 -c 10
     dnstracer compare -d google.com -f tests/dns.txt -c 10 -o json
     dnstracer compare -d google.com -c 5 -w 30
 
   Happy debugging!
 "#, env!("CARGO_PKG_VERSION")
     );
    std::process::exit(1);
}

fn ping(domain: &str, dns_server: &str, interval: u64, count: u32, show_plot: bool, qtype: QueryType, timeout: Duration) -> io::Result<()> {
    let (response_times, failed) = perform_dns_queries(domain, dns_server, count, interval, true, qtype, timeout)?;
    let stats = calculate_statistics(&response_times);

    print_statistics(domain, count, failed, &stats);

    if show_plot {
        plot_response_times(&response_times);
    }

    Ok(())
}

fn display_server(s: &str) -> String {
    let s = s.strip_suffix(":53").unwrap_or(s);
    if s.starts_with('[') && s.ends_with(']') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn normalize_server(s: &str) -> String {
    if s.starts_with('[') {
        s.to_string() // already [IPv6]:port
    } else if s.chars().filter(|&c| c == ':').count() > 1 {
        format!("[{}]:53", s) // bare IPv6, add brackets + port
    } else if s.contains(':') {
        s.to_string() // IPv4 with port
    } else {
        format!("{}:53", s) // IPv4 without port
    }
}

fn spectrum_color(t: f64) -> (u8, u8, u8) {
    let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
    let (r, g, b) = if t < 0.5 {
        let t2 = t * 2.0;
        (lerp(0.0, 220.0, t2), lerp(200.0, 180.0, t2), 0.0)
    } else {
        let t2 = (t - 0.5) * 2.0;
        (220.0, lerp(180.0, 0.0, t2), 0.0)
    };
    (r as u8, g as u8, b as u8)
}

fn compare(domain: &str, dns_file: &str, interval: u64, count: u32, qtype: QueryType, sort_by: SortBy, timeout: Duration, out_fmt: OutputFormat, watch_secs: u64) -> io::Result<()> {
    let reader: Box<dyn BufRead> = if dns_file.is_empty() {
        Box::new(BufReader::new(Cursor::new(DEFAULT_DNS_SERVERS.join("\n"))))
    } else {
        Box::new(BufReader::new(File::open(dns_file)?))
    };

    let servers: Vec<String> = reader.lines()
        .collect::<io::Result<Vec<String>>>()?
        .into_iter()
        .map(|s| normalize_server(&s))
        .collect();

    let is_tty = io::stdout().is_terminal();
    let watch_mode = watch_secs > 0;

    loop {
        let streaming = matches!(out_fmt, OutputFormat::Text) && is_tty && !watch_mode;

        if streaming {
            print!("\x1b[?1049h");
            println!("{:<25} {:<10} {:<10} {:<10} {:<12} {:<10}",
                "server", "min(ms)", "avg(ms)", "max(ms)", "stddev(ms)", "lost(%)");
            println!("{:-<77}", "");
        }

        let (work_tx, work_rx) = mpsc::channel::<String>();
        let work_rx = Arc::new(Mutex::new(work_rx));
        let (result_tx, result_rx) = mpsc::channel::<ResultRow>();

        let num_workers = CONCURRENCY_LIMIT.min(servers.len());
        let mut handles = Vec::with_capacity(num_workers);

        for _ in 0..num_workers {
            let work_rx = Arc::clone(&work_rx);
            let result_tx = result_tx.clone();
            let domain = domain.to_string();

            handles.push(thread::spawn(move || {
                loop {
                    let server = match work_rx.lock().unwrap().recv() {
                        Ok(item) => item,
                        Err(_) => break,
                    };
                    let row = match perform_dns_queries(&domain, &server, count, interval, false, qtype, timeout) {
                        Ok((times, failed)) => {
                            let stats = calculate_statistics(&times);
                            let loss_pct = (failed as f64 / count as f64) * 100.0;
                            ResultRow { server, min: stats.0, avg: stats.1, max: stats.2, stddev: stats.3, loss: loss_pct, is_error: false }
                        }
                        Err(_) => ResultRow { server, min: 0.0, avg: 0.0, max: 0.0, stddev: 0.0, loss: 100.0, is_error: true },
                    };
                    if result_tx.send(row).is_err() { break; }
                }
            }));
        }

        drop(result_tx);

        for server in &servers {
            work_tx.send(server.clone()).unwrap();
        }
        drop(work_tx);

        let mut all_results: Vec<ResultRow> = Vec::with_capacity(servers.len());
        for row in result_rx {
            if streaming { println!("{}", row.to_text()); }
            all_results.push(row);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        all_results.sort_by(|a, b| {
            a.sort_key(sort_by).partial_cmp(&b.sort_key(sort_by))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        match out_fmt {
            OutputFormat::Text => {
                if streaming {
                    print!("\x1b[?1049l");
                } else if watch_mode && is_tty {
                    print!("\x1b[2J\x1b[H");
                } else if !is_tty && !watch_mode {
                    println!();
                }

                println!("{:<25} {:<10} {:<10} {:<10} {:<12} {:<10}",
                    "server", "min(ms)", "avg(ms)", "max(ms)", "stddev(ms)", "lost(%)");
                println!("{:-<77}", "");
                let n = all_results.len();
                for (i, row) in all_results.iter().enumerate() {
                    if is_tty && n > 1 && !row.is_error {
                        let t = i as f64 / (n - 1) as f64;
                        let (r, g, b) = spectrum_color(t);
                        println!("\x1b[38;2;{};{};{}m{}\x1b[0m", r, g, b, row.to_text());
                    } else {
                        println!("{}", row.to_text());
                    }
                }

                if watch_mode {
                    println!("\nRefreshing every {}s — Ctrl+C to quit", watch_secs);
                    thread::sleep(Duration::from_secs(watch_secs));
                }
            }
            OutputFormat::Json => {
                if streaming { print!("\x1b[?1049l"); }
                let items: Vec<String> = all_results.iter().map(|r| r.to_json()).collect();
                println!("[{}]", items.join(","));
                if watch_mode {
                    thread::sleep(Duration::from_secs(watch_secs));
                }
            }
        }

        if !watch_mode { break; }
    }

    Ok(())
}

fn perform_dns_queries(domain: &str, dns_server: &str, count: u32, interval: u64, verbose: bool, qtype: QueryType, timeout: Duration) -> io::Result<(Vec<f64>, u32)> {
    let packet = create_dns_query(domain, qtype);
    let bind_addr = if dns_server.starts_with('[') { "[::]:0" } else { "0.0.0.0:0" };
    let socket = UdpSocket::bind(bind_addr)?;
    socket.set_read_timeout(Some(timeout))?;

    let mut response_times = Vec::new();
    let mut failed = 0;

    for c in 0..count {
        let start = Instant::now();
        if socket.send_to(&packet, dns_server).is_err() {
            if verbose {
                println!("seq={:<10} Failed to send data", c);
            }
            failed += 1;
            response_times.push(-1.0);
            continue;
        }

        let mut buf = [0; MAX_RESPONSE_SIZE];
        match socket.recv_from(&mut buf) {
            Ok((resp_size, _src)) => {
                let duration = start.elapsed();
                let duration_ms = duration.as_secs_f64() * 1000.0;
                response_times.push(duration_ms);

                let response = &buf[..resp_size];
                if verbose {
                    match parse_dns_response(response) {
                        Ok(ip_addr) => {
                            println!(
                                "{} bytes from {} seq={} time={:.3}ms - {} -> {}",
                                resp_size,
                                display_server(dns_server),
                                c,
                                duration_ms,
                                domain,
                                ip_addr
                            );
                        }
                        Err(e) => println!("seq={:<10} Failed to parse response: {}", c, e),
                    }
                }
            }
            Err(e) => {
                if verbose {
                    println!("seq={:<10} Failed to receive data: {}", c, e);
                }
                failed += 1;
                response_times.push(-1.0);
            }
        }

        thread::sleep(Duration::from_secs(interval));
    }

    Ok((response_times, failed))
}

fn create_dns_query(domain: &str, qtype: QueryType) -> Vec<u8> {
    let mut packet = vec![
        0x12, 0x12,
        0x01, 0x00,
        0x00, 0x01,
        0x00, 0x00,
        0x00, 0x00,
        0x00, 0x00,
    ];

    for part in domain.split('.') {
        packet.push(part.len() as u8);
        packet.extend_from_slice(part.as_bytes());
    }
    packet.push(0x00);
    packet.extend_from_slice(&qtype.type_code()); // QTYPE
    packet.extend_from_slice(&[0x00, 0x01]);       // QCLASS IN

    packet
}


fn parse_dns_response(response: &[u8]) -> io::Result<String> {
    if response.len() < 12 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "response too short"));
    }

    // Skip the question section name (starts at byte 12)
    let mut offset = 12;
    loop {
        if offset >= response.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "response truncated in question section"));
        }
        let label_len = response[offset] as usize;
        if label_len == 0 {
            break;
        }
        offset += label_len + 1;
    }
    // Skip null byte + QTYPE (2) + QCLASS (2)
    offset += 5;

    let ancount = (response[6] as usize) << 8 | (response[7] as usize);
    let mut result = String::new();

    for _ in 0..ancount {
        // Skip the answer's name field
        if offset >= response.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "response truncated in answer name"));
        }
        if response[offset] & 0xC0 == 0xC0 {
            offset += 2;
        } else {
            loop {
                if offset >= response.len() {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "response truncated in answer name"));
                }
                let label_len = response[offset] as usize;
                if label_len == 0 {
                    offset += 1;
                    break;
                }
                offset += label_len + 1;
            }
        }

        // TYPE(2) + CLASS(2) + TTL(4) + RDLENGTH(2) = 10 bytes
        if offset + 10 > response.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "response truncated in answer record"));
        }
        let answer_type = &response[offset..offset + 2];
        let answer_class = &response[offset + 2..offset + 4];
        let answer_data_len = (response[offset + 8] as usize) << 8 | (response[offset + 9] as usize);

        if offset + 10 + answer_data_len > response.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "response truncated in answer data"));
        }
        let rdata_start = offset + 10;
        let answer_data = &response[rdata_start..rdata_start + answer_data_len];
        offset = rdata_start + answer_data_len;

        if answer_type == [0x00, 0x01] && answer_class == [0x00, 0x01] {
            // A record: 4-byte IPv4 address
            if answer_data_len == 4 {
                result.push_str(&format!("{}.{}.{}.{} ", answer_data[0], answer_data[1], answer_data[2], answer_data[3]));
            }
        } else if answer_type == [0x00, 0x1C] && answer_class == [0x00, 0x01] {
            // AAAA record: 16-byte IPv6 address
            if answer_data_len == 16 {
                let addr = (0..8)
                    .map(|i| format!("{:04x}", (answer_data[i * 2] as u16) << 8 | answer_data[i * 2 + 1] as u16))
                    .collect::<Vec<_>>()
                    .join(":");
                result.push_str(&format!("{} ", addr));
            }
        } else if answer_type == [0x00, 0x10] && answer_class == [0x00, 0x01] {
            // TXT record: one or more length-prefixed strings
            let mut off = 0;
            while off < answer_data_len {
                let slen = answer_data[off] as usize;
                off += 1;
                if off + slen <= answer_data_len {
                    if let Ok(s) = std::str::from_utf8(&answer_data[off..off + slen]) {
                        result.push_str(s);
                    }
                }
                off += slen;
            }
            result.push(' ');
        } else if answer_type == [0x00, 0x02] && answer_class == [0x00, 0x01] {
            // NS record: domain name in wire format
            let mut ns_name = String::new();
            parse_name(response, rdata_start, &mut ns_name)?;
            result.push_str(&format!("{} ", ns_name));
        } else if answer_type == [0x00, 0x05] && answer_class == [0x00, 0x01] {
            let mut cname = String::new();
            let mut cname_offset = 0;
            while cname_offset < answer_data_len {
                let label_len = answer_data[cname_offset] as usize;
                if label_len & 0xC0 == 0xC0 {
                    if cname_offset + 2 > answer_data_len {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "CNAME pointer out of bounds"));
                    }
                    let pointer = ((label_len & 0x3F) << 8) | (answer_data[cname_offset + 1] as usize);
                    parse_name(response, pointer, &mut cname)?;
                    break; // pointer terminates the name
                } else {
                    if label_len == 0 {
                        break;
                    }
                    if !cname.is_empty() {
                        cname.push('.');
                    }
                    if cname_offset + 1 + label_len > answer_data_len {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "CNAME label length exceeds data length"));
                    }
                    cname.push_str(std::str::from_utf8(&answer_data[cname_offset + 1..cname_offset + 1 + label_len])
                        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid UTF-8 in CNAME"))?);
                    cname_offset += label_len + 1;
                }
            }
            result.push_str(&format!("{} -> ", cname));
        }
    }

    if result.is_empty() {
        Err(io::Error::new(io::ErrorKind::InvalidData, "No matching records found in response"))
    } else {
        Ok(result)
    }
}


fn parse_name(response: &[u8], mut offset: usize, name: &mut String) -> io::Result<()> {
    let mut hops = 0;
    loop {
        if hops > 10 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "DNS name compression pointer loop"));
        }
        if offset >= response.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "response truncated in name"));
        }
        let label_len = response[offset] as usize;
        if label_len & 0xC0 == 0xC0 {
            if offset + 2 > response.len() {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "response truncated in name pointer"));
            }
            let pointer = ((label_len & 0x3F) << 8) | (response[offset + 1] as usize);
            offset = pointer;
            hops += 1;
        } else {
            if label_len == 0 {
                break;
            }
            if !name.is_empty() {
                name.push('.');
            }
            if offset + 1 + label_len > response.len() {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "label length exceeds response length"));
            }
            name.push_str(std::str::from_utf8(&response[offset + 1..offset + 1 + label_len])
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid UTF-8 in DNS name"))?);
            offset += label_len + 1;
        }
    }
    Ok(())
}


fn calculate_statistics(response_times: &[f64]) -> (f64, f64, f64, f64) {
    let valid_times: Vec<&f64> = response_times.iter().filter(|&&x| x != -1.0).collect();
    if valid_times.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let avg = valid_times.iter().copied().sum::<f64>() / valid_times.len() as f64;
    let min = valid_times.iter().copied().fold(f64::INFINITY, |a, b| a.min(*b));
    let max = valid_times.iter().copied().fold(f64::NEG_INFINITY, |a, b| a.max(*b));
    let std_dev = (valid_times.iter().map(|&&t| (t - avg).powi(2)).sum::<f64>() / valid_times.len() as f64).sqrt();

    (min, avg, max, std_dev)
}


fn print_statistics(domain: &str, count: u32, failed: u32, stats: &(f64, f64, f64, f64)) {
    let (min, avg, max, std_dev) = stats;

    println!("\n--- {} dns query statistics ---", domain);
    println!("{} queries transmitted, {} responses received, {:.1}% data loss", count, count - failed, (failed as f64 / count as f64) * 100.0);
    println!("Response time min/avg/max/stddev = {:.3}/{:.3}/{:.3}/{:.3} ms", min, avg, max, std_dev);
}

fn plot_response_times(response_times: &[f64]) {
    let max_time = response_times.iter().cloned().filter(|&t| t >= 0.0).fold(f64::NEG_INFINITY, f64::max);
    let plot_height = 10;

    for (i, &time) in response_times.iter().enumerate() {
        if time < 0.0 {
            println!("{:3}: timeout", i + 1);
        } else {
            let scale = plot_height as f64 / max_time;
            let bar_height = (time * scale).round() as usize;
            let bar: String = std::iter::repeat('#').take(bar_height).collect();
            println!("{:3}: {} {:.3} ms", i + 1, bar, time);
        }
    }
}
