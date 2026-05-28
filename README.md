# DNSTracer
  A tool to measure and analyze DNS query response times for network performance and latency.

  ```
  $ dnstracer ping -d raw.githubusercontent.com -s 1.0.0.1:53 -i 1 -c 5 -p

  107 bytes from 1.0.0.1:53 seq=0 time=129.566ms - raw.githubusercontent.com -> 185.199.109.133 185.199.108.133 185.199.111.133 185.199.110.133
  107 bytes from 1.0.0.1:53 seq=1 time=122.775ms - raw.githubusercontent.com -> 185.199.109.133 185.199.111.133 185.199.108.133 185.199.110.133
  107 bytes from 1.0.0.1:53 seq=2 time=145.766ms - raw.githubusercontent.com -> 185.199.109.133 185.199.108.133 185.199.110.133 185.199.111.133
  107 bytes from 1.0.0.1:53 seq=3 time=134.356ms - raw.githubusercontent.com -> 185.199.111.133 185.199.108.133 185.199.110.133 185.199.109.133
  107 bytes from 1.0.0.1:53 seq=4 time=135.027ms - raw.githubusercontent.com -> 185.199.110.133 185.199.108.133 185.199.111.133 185.199.109.133

  --- raw.githubusercontent.com dns query statistics ---
  5 queries transmitted, 5 responses received, 0.0% data loss
  Response time min/avg/max/stddev = 122.775/133.498/145.766/7.894 ms
    1: ######### 129.566 ms
    2: ######## 122.775 ms
    3: ########## 145.766 ms
    4: ######### 134.356 ms
    5: ######### 135.027 ms
  ```

  ```
  $ dnstracer compare -d github.blog -i 5 -c 3

  (results stream in as each server finishes, then final sorted table appears)

  --- sorted by avg ---
  server                    min(ms)    avg(ms)    max(ms)    stddev(ms)   lost(%)
  -----------------------------------------------------------------------------
  1.0.0.1:53                100.433    109.580    116.509    6.015        0.0
  156.154.71.1:53           101.527    106.964    115.673    5.209        0.0
  9.9.9.9:53                98.537     113.650    119.981    7.915        0.0
  1.1.1.1:53                100.251    113.971    126.046    9.747        0.0
  9.9.9.10:53               101.397    112.777    120.072    6.956        0.0
  8.8.8.8:53                129.236    150.126    167.230    14.369       0.0
  8.8.4.4:53                157.800    163.125    168.753    4.211        0.0
  208.67.222.222:53         151.765    165.201    182.040    10.533       0.0
  208.67.220.220:53         133.202    178.055    345.618    83.826       0.0
  ```

  ## Usage:

  - ***Query a DNS server and measure response times***

    ```
    dnstracer ping -d <domain> -s <server> [-i <seconds>] [-c <n>] [-T <seconds>] [-p] [-t A|AAAA|TXT|NS]
        - -d, --domain:    The domain name to query.
        - -s, --server:    DNS server — IPv4 (1.1.1.1), IPv6 (2606:4700::1111), or with port (1.1.1.1:5353).
        - -i, --interval:  Time in seconds between each query (default: 1).
        - -c, --count:     Number of queries to perform (default: 5).
        - -T, --timeout:   Query timeout in seconds (default: 5).
        - -p, --plot:      Display a plot of response times.
        - -t, --type:      Query type (default: A). Supported: A, AAAA, TXT, NS.

    Examples:
    dnstracer ping -d google.com -s 1.1.1.1 -i 5 -c 10 -p
    dnstracer ping -d google.com -s 2606:4700:4700::1111 -c 10 -t AAAA
    ```

  - ***Compare multiple DNS servers***

    Results stream live as each server finishes. When all are done the screen clears and a final sorted table is shown.

    ```
    dnstracer compare -d <domain> [-f <dns_file>] [-i <seconds>] [-c <n>] [-T <seconds>] [-t A|AAAA|TXT|NS] [-S min|avg|max|stddev|loss] [-o text|json] [-w <seconds>]
        - -d, --domain:    The domain name to query.
        - -f, --file:      Path to a file containing DNS servers (uses built-in list of 42 servers if omitted).
        - -i, --interval:  Time in seconds between each query (default: 1).
        - -c, --count:     Number of queries to perform (default: 5).
        - -T, --timeout:   Query timeout in seconds (default: 5).
        - -t, --type:      Query type (default: A). Supported: A, AAAA, TXT, NS.
        - -S, --sort:      Sort results by field (default: avg). Supported: min, avg, max, stddev, loss.
        - -o, --output:    Output format (default: text). Supported: text, json.
        - -w, --watch:     Re-run every N seconds and refresh the screen.

    Examples:
    dnstracer compare -d google.com -i 5 -c 10
    dnstracer compare -d google.com -f tests/dns.txt -c 10 -t AAAA
    dnstracer compare -d google.com -f tests/dns.txt -c 10 -S loss
    dnstracer compare -d google.com -c 5 -o json
    dnstracer compare -d google.com -c 5 -w 30
    ```

## Installation
- ***Download pre-built binaries:***

  Pre-built binaries are available on the [releases page.](https://github.com/miladbr/dnstracer/releases)

- ***Build from source***:

  ```
  $ git clone https://github.com/miladbr/dnstracer.git
  $ cd dnstracer
  $ cargo build --release
  ```

- ***Build container image***:

  ```
  $ git clone https://github.com/miladbr/dnstracer.git
  $ cd dnstracer
  $ docker build . -t dnstracer
  ```
