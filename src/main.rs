use std::env;
use std::fs::File;
use std::io::{self, Cursor, BufReader, BufRead};
use std::net::UdpSocket;
use std::time::{Duration, Instant};
use std::thread::sleep;
use std::f64;

const TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RESPONSE_SIZE: usize = 512;
const DEFAULT_DNS_SERVERS: [&str; 24] = [
    "9.9.9.11:53",       // Quad9
    "149.112.112.11:53", // Quad9
    "9.9.9.10:53",       // Quad9
    "149.112.112.10:53", // Quad9
    "1.1.1.1:53",        // Cloudflare
    "1.0.0.1:53",        // Cloudflare
    "8.8.8.8:53",        // Google
    "8.8.4.4:53",        // Google
    "9.9.9.9:53",        // Quad9
    "149.112.112.112:53",// Quad9
    "208.67.222.222:53", // OpenDNS
    "208.67.220.220:53", // OpenDNS
    "64.6.64.6:53",      // Verisign
    "64.6.65.6:53",      // Verisign
    "8.26.56.26:53",     // Comodo Secure DNS
    "8.20.247.20:53",    // Comodo Secure DNS
    "77.88.8.8:53",      // Yandex DNS
    "77.88.8.1:53",      // Yandex DNS
    "185.228.168.168:53",// CleanBrowsing
    "185.228.169.168:53",// CleanBrowsing
    "156.154.70.1:53",   // Neustar UltraDNS
    "156.154.71.1:53",   // Neustar UltraDNS
    "199.85.126.10:53",  // Norton ConnectSafe
    "199.85.127.10:53",  // Norton ConnectSafe
];

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage_and_exit();
    }

    let subcommand = &args[1];
    match subcommand.as_str() {
        "ping" => {
            if args.len() != 7 {
                print_usage_and_exit();
            }
            let domain = &args[2];
            let dns_server = &args[3];
            let interval = args[4].parse::<u64>().expect("Interval must be a number");
            let count = args[5].parse::<u32>().expect("Count must be a number");
            let show_plot = args[6].parse::<bool>().expect("Show plot must be a boolean (true or false)");

            ping(domain, dns_server, interval, count, show_plot)?;
        }
        "compare" => {
            if args.len() < 5 || args.len() > 6 {
                print_usage_and_exit();
            }
            let domain = &args[2];
            let (dns_file, interval, count) = if args.len() == 6 {
                (
                    &args[3] as &str,
                    args[4].parse::<u64>().expect("Interval must be a number"),
                    args[5].parse::<u32>().expect("Count must be a number"),
                )
            } else {
                (
                    "",
                    args[3].parse::<u64>().expect("Interval must be a number"),
                    args[4].parse::<u32>().expect("Count must be a number"),
                )
            };
            compare(domain, dns_file, interval, count)?;
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

   DNS Tracer Tool v0.1
   A tool to measure and analyze DNS query response times for network performance and latency.

   Developed by: @milad_bahari
 
   USAGE:
     ./dnstracer ping <domain> <dns_server> <interval> <count> <show_plot>
       - domain:     The domain name to query.
       - dns_server: The DNS server to use (e.g., 1.1.1.1:53).
       - interval:   Time in seconds between each query.
       - count:      Number of queries to perform.
       - show_plot:  Set to 'true' to display a plot of the response times.
 
     ./dnstracer compare <domain> <dns_file> <interval> <count>
       - domain:     The domain name to query.
       - dns_file:   Path to a file containing a list of DNS servers.
       - interval:   Time in seconds between each query.
       - count:      Number of queries to perform.
   
   EXAMPLES:
     ./dnstracer ping google.com 1.1.1.1:53 5 10 true
     ./dnstracer compare google.com tests/dns.txt 5 10
 
   Happy debugging!
 "#
     );
    std::process::exit(1);
}

fn ping(domain: &str, dns_server: &str, interval: u64, count: u32, show_plot: bool) -> io::Result<()> {
    let (response_times, failed) = perform_dns_queries(domain, dns_server, count, interval, true)?;
    let stats = calculate_statistics(&response_times);

    print_statistics(domain, count, failed, &stats);

    if show_plot {
        plot_response_times(&response_times);
    }

    Ok(())
}

fn compare(domain: &str, dns_file: &str, interval: u64, count: u32) -> io::Result<()> {
    let reader: Box<dyn BufRead> = if dns_file.is_empty() {
        // Use the default DNS servers as an in-memory buffer
        Box::new(BufReader::new(Cursor::new(DEFAULT_DNS_SERVERS.join("\n"))))
    } else {
        // Open and read from the specified file
        Box::new(BufReader::new(File::open(dns_file)?))
    };

    println!("{:<25} {:<10} {:<10} {:<10} {:<12} {:<10}", 
        "server", "min(ms)", "avg(ms)", "max(ms)", "stddev(ms)", "lost(%)");
    println!("{:-<77}", "");

    for line in reader.lines() {
        let dns_server = line?;
        let (response_times, failed) = perform_dns_queries(domain, &dns_server, count, interval, false)?;
        let stats = calculate_statistics(&response_times);

        let result = generate_compare_output(&dns_server, count, failed, &stats)?;
        println!("{}", result);
    }

    Ok(())
}

fn perform_dns_queries(domain: &str, dns_server: &str, count: u32, interval: u64, verbose: bool) -> io::Result<(Vec<f64>, u32)> {
    let packet = create_dns_query(domain);
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_read_timeout(Some(TIMEOUT))?;

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
                                dns_server,
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

        sleep(Duration::from_secs(interval));
    }

    Ok((response_times, failed))
}

fn create_dns_query(domain: &str) -> Vec<u8> {
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
    packet.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]);

    packet
}


fn parse_dns_response(response: &[u8]) -> io::Result<String> {
    let mut offset = 12;
    while response[offset] != 0 {
        offset += response[offset] as usize + 1;
    }
    offset += 5;

    let mut result = String::new();
    let ancount = (response[6] as usize) << 8 | (response[7] as usize);

    for _ in 0..ancount {
        if response[offset] & 0xC0 == 0xC0 {
            offset += 2; // Skip the name pointer
        } else {
            while response[offset] != 0 {
                offset += response[offset] as usize + 1;
            }
            offset += 1; // Skip the null byte
        }

        let answer_type = &response[offset..offset + 2];
        let answer_class = &response[offset + 2..offset + 4];
        let answer_data_len = (response[offset + 8] as usize) << 8 | (response[offset + 9] as usize);
        let answer_data = &response[offset + 10..offset + 10 + answer_data_len];
        offset += 10 + answer_data_len;

        if answer_type == [0x00, 0x01] && answer_class == [0x00, 0x01] {
            if answer_data_len == 4 {
                let ip_addr = format!("{}.{}.{}.{}", answer_data[0], answer_data[1], answer_data[2], answer_data[3]);
                result.push_str(&format!("{} ", ip_addr));
            }
        } else if answer_type == [0x00, 0x05] && answer_class == [0x00, 0x01] {
            let mut cname = String::new();
            let mut cname_offset = 0;
            while cname_offset < answer_data_len {
                let label_len = answer_data[cname_offset] as usize;
                if label_len & 0xC0 == 0xC0 {
                    // It is a pointer
                    let pointer = ((label_len & 0x3F) << 8) | (answer_data[cname_offset + 1] as usize);
                    cname_offset += 2;
                    parse_name(&response, pointer, &mut cname)?;
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
                    cname.push_str(std::str::from_utf8(&answer_data[cname_offset + 1..cname_offset + 1 + label_len]).unwrap());
                    cname_offset += label_len + 1;
                }
            }
            result.push_str(&format!("{} -> ", cname));
        }
    }

    if result.is_empty() {
        Err(io::Error::new(io::ErrorKind::InvalidData, "No A or CNAME records found"))
    } else {
        Ok(result)
    }
}


fn parse_name(response: &[u8], mut offset: usize, name: &mut String) -> io::Result<()> {
    loop {
        let label_len = response[offset] as usize;
        if label_len & 0xC0 == 0xC0 {
            // It is a pointer
            let pointer = ((label_len & 0x3F) << 8) | (response[offset + 1] as usize);
            offset = pointer;
        } else {
            if label_len == 0 {
                break;
            }
            if !name.is_empty() {
                name.push('.');
            }
            if offset + 1 + label_len > response.len() {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Label length exceeds data length"));
            }
            name.push_str(std::str::from_utf8(&response[offset + 1..offset + 1 + label_len]).unwrap());
            offset += label_len + 1;
        }
    }
    Ok(())
}


fn calculate_statistics(response_times: &[f64]) -> (f64, f64, f64, f64) {
    let valid_times: Vec<&f64> = response_times.iter().filter(|&&x| x != -1.0).collect();
    let avg = valid_times.iter().copied().sum::<f64>() / valid_times.len() as f64;
    let min = valid_times.iter().copied().fold(f64::INFINITY, |a, b| a.min(*b));
    let max = valid_times.iter().copied().fold(f64::NEG_INFINITY, |a, b| a.max(*b));
    let std_dev = (valid_times.iter().map(|&&t| (t - avg).powi(2)).sum::<f64>() / valid_times.len() as f64).sqrt();

    (min, avg, max, std_dev)
}

fn generate_compare_output(dns_server: &str, count: u32, failed: u32, stats: &(f64, f64, f64, f64)) -> io::Result<String> {
    let (min, avg, max, std_dev) = stats;
    let loss_percentage = (failed as f64 / count as f64) * 100.0;

    Ok(format!("{:<25} {:<10.3} {:<10.3} {:<10.3} {:<12.3} {:<10.1}", 
        dns_server, min, avg, max, std_dev, loss_percentage))
}

fn print_statistics(domain: &str, count: u32, failed: u32, stats: &(f64, f64, f64, f64)) {
    let (min, avg, max, std_dev) = stats;

    println!("\n--- {} dns query statistics ---", domain);
    println!("{} queries transmitted, {} responses received, {:.1}% data loss", count, count - failed, (failed as f64 / count as f64) * 100.0);
    println!("Response time min/avg/max/stddev = {:.3}/{:.3}/{:.3}/{:.3} ms", min, avg, max, std_dev);
}

fn plot_response_times(response_times: &[f64]) {
    let max_time = response_times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let plot_height = 10;
    let scale = plot_height as f64 / max_time;

    for (i, &time) in response_times.iter().enumerate() {
        let bar_height = (time * scale).round() as usize;
        let bar: String = std::iter::repeat('#').take(bar_height).collect();
        println!("{:3}: {} {:.3} ms", i + 1, bar, time);
    }
}
