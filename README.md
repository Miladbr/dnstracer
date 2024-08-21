# DNSTracer
  A tool to measure and analyze DNS query response times for network performance and latency.

  ```
  $ dnstracer ping raw.githubusercontent.com 1.0.0.1:53 5 10 true

  107 bytes from 1.0.0.1:53 seq=0 time=129.566ms - raw.githubusercontent.com -> 185.199.109.133 185.199.108.133 185.199.111.133 185.199.110.133
  107 bytes from 1.0.0.1:53 seq=1 time=122.775ms - raw.githubusercontent.com -> 185.199.109.133 185.199.111.133 185.199.108.133 185.199.110.133
  107 bytes from 1.0.0.1:53 seq=2 time=145.766ms - raw.githubusercontent.com -> 185.199.109.133 185.199.108.133 185.199.110.133 185.199.111.133
  107 bytes from 1.0.0.1:53 seq=3 time=134.356ms - raw.githubusercontent.com -> 185.199.111.133 185.199.108.133 185.199.110.133 185.199.109.133
  107 bytes from 1.0.0.1:53 seq=4 time=135.027ms - raw.githubusercontent.com -> 185.199.110.133 185.199.108.133 185.199.111.133 185.199.109.133
  107 bytes from 1.0.0.1:53 seq=5 time=127.134ms - raw.githubusercontent.com -> 185.199.109.133 185.199.111.133 185.199.110.133 185.199.108.133
  107 bytes from 1.0.0.1:53 seq=6 time=123.815ms - raw.githubusercontent.com -> 185.199.109.133 185.199.111.133 185.199.110.133 185.199.108.133
  107 bytes from 1.0.0.1:53 seq=7 time=134.091ms - raw.githubusercontent.com -> 185.199.109.133 185.199.111.133 185.199.108.133 185.199.110.133
  107 bytes from 1.0.0.1:53 seq=8 time=124.340ms - raw.githubusercontent.com -> 185.199.108.133 185.199.110.133 185.199.111.133 185.199.109.133
  107 bytes from 1.0.0.1:53 seq=9 time=127.814ms - raw.githubusercontent.com -> 185.199.109.133 185.199.111.133 185.199.110.133 185.199.108.133

  --- raw.githubusercontent.com dns query statistics ---
  10 queries transmitted, 10 responses received, 0.0% data loss
  Response time min/avg/max/stddev = 122.775/130.469/145.766/6.656 ms
    1: ######### 129.566 ms
    2: ######## 122.775 ms
    3: ########## 145.766 ms
    4: ######### 134.356 ms
    5: ######### 135.027 ms
    6: ######### 127.134 ms
    7: ######## 123.815 ms
    8: ######### 134.091 ms
    9: ######### 124.340 ms
  10: ######### 127.814 ms
  ```

  ```
  $ dnstracer compare github.blog 15 5

  server                    min(ms)    avg(ms)    max(ms)    stddev(ms)   lost(%)
  -----------------------------------------------------------------------------
  9.9.9.11:53               102.795    110.074    114.795    4.286        0.0
  149.112.112.11:53         107.863    115.942    132.772    9.834        20.0
  9.9.9.10:53               101.397    112.777    120.072    6.956        0.0
  149.112.112.10:53         105.521    108.729    115.830    3.750        0.0
  1.1.1.1:53                100.251    113.971    126.046    9.747        0.0
  1.0.0.1:53                100.433    109.580    116.509    6.015        0.0
  8.8.8.8:53                129.236    150.126    167.230    14.369       0.0
  8.8.4.4:53                157.800    163.125    168.753    4.211        0.0
  9.9.9.9:53                98.537     113.650    119.981    7.915        0.0
  149.112.112.112:53        113.864    119.512    125.236    4.823        20.0
  208.67.222.222:53         151.765    165.201    182.040    10.533       0.0
  208.67.220.220:53         133.202    178.055    345.618    83.826       0.0
  64.6.64.6:53              103.790    111.483    121.067    7.040        0.0
  64.6.65.6:53              103.873    113.309    119.295    6.238        0.0
  8.26.56.26:53             108.074    116.205    131.490    8.386        0.0
  8.20.247.20:53            105.536    111.775    121.930    6.021        0.0
  77.88.8.8:53              110.150    146.085    214.389    36.147       0.0
  77.88.8.1:53              116.838    136.122    205.176    34.579       0.0
  185.228.168.168:53        99.352     109.963    124.283    9.512        0.0
  185.228.169.168:53        104.535    113.118    127.521    9.153        0.0
  156.154.70.1:53           101.929    109.895    114.717    5.290        0.0
  156.154.71.1:53           101.527    106.964    115.673    5.209        0.0
  199.85.126.10:53          102.240    106.287    112.155    3.814        0.0
  199.85.127.10:53          108.053    119.514    127.865    7.690        0.0
  ```

  ## Usage:

  - ***Send DNS A record query to the specified DNS server***

    ```
    dnstracer ping <domain> <dns_server> <interval> <count> <show_plot>
        - domain:     The domain name to query.
        - dns_server: The DNS server to use (e.g., 1.1.1.1:53).
        - interval:   Time in seconds between each query.
        - count:      Number of queries to perform.
        - show_plot:  Set to 'true' to display a plot of the response times.

    Example:
    dnstracer ping raw.githubusercontent.com 1.1.1.1:53 5 10 true
    ```

  - ***Compare multiple DNS servers***

    ```
    dnstracer compare <domain> [dns_file] <interval> <count>
        - domain:     The domain name to query.
        - dns_file[Optional]:   Path to a file containing a list of DNS servers. If not provided dnstracer uses embeded DNS list.
        - interval:   Time in seconds between each query.
        - count:      Number of queries to perform.

    Example:
    dnstracer compare raw.githubusercontent.com 5 10
    ```
## Installation
- ***Download pre-built binaries:***

  Pre-built binaries are available on the [releases page.](https://github.com/miladbr/dnstracer/releases)

- ***Build from source***:

  ```
  $ git clone https://github.com/miladbr/dnstracer.git
  $ cd dnstrace
  $ cargo build
  ```
- ***Build container image***:

  ```
  $ git clone https://github.com/miladbr/dnstracer.git
  $ cd dnstrace
  $ docker build . -t dnstracer
  ```
