# rollping

A fast, concurrent ping statistics aggregator with built-in geolocation support.

`rollping` reads a list of hosts from stdin, pings them concurrently, and outputs aggregated statistics as single-line JSON. Perfect for monitoring network latency across multiple hosts and integrating with other command-line tools.

## Features

- **Concurrent pinging** - Tests multiple hosts simultaneously for fast results
- **Statistical analysis** - Calculates avg, median, p95, p99, and max latency
- **Geolocation** - Automatically detects and reports your location using MaxMind GeoLite2
- **Composable output** - Single-line JSON output for easy piping to `jq`, log files, etc.
- **Configurable logging** - Respects `RUST_LOG` environment variable
- **Minimal overhead** - Efficient Rust implementation with tokio async runtime

## Installation

### From crates.io

```bash
cargo install --locked rollping
```

### From source

```bash
git clone https://github.com/wbbradley/rollping
cd rollping
cargo install --path .
```

## Usage

### Basic Example

```bash
echo -e "8.8.8.8\n1.1.1.1\n1.0.0.1" | rollping
```

### With Custom Settings

```bash
# Send 5 pings to each host with 1 second timeout
echo -e "8.8.8.8\n1.1.1.1" | rollping -c 5 -t 1.0
```

### From a File

```bash
cat hosts.txt | rollping -c 3
```

### Silent Mode (JSON only)

```bash
echo -e "8.8.8.8\n1.1.1.1" | RUST_LOG=error rollping
```

### Pretty Print with jq

```bash
echo -e "8.8.8.8\n1.1.1.1" | RUST_LOG=error rollping | jq .
```

## Output Format

The output is a single-line JSON object with the following fields:

```json
{
  "avg_ms": 4.23,
  "median_ms": 4.17,
  "p95_ms": 4.63,
  "p99_ms": 4.63,
  "max_ms": 4.63,
  "non_responsive_nodes": 0,
  "total_hosts": 2,
  "pings_per_host": 3,
  "timeout_secs": 2.0,
  "location": {
    "country": "United States",
    "country_code": "US",
    "city": "Denver",
    "latitude": 39.8661,
    "longitude": -104.9197
  }
}
```

## Options

```
Options:
  -c, --count <COUNT>
          Number of pings to send to each host [default: 3]

  -t, --timeout-secs <TIMEOUT_SECS>
          Timeout in seconds for each ping [default: 2.0]

  -h, --help
          Print help

  -V, --version
          Print version
```

## Logging

`rollping` uses the `RUST_LOG` environment variable to control log output to stderr:

```bash
# Show all logs including debug info
RUST_LOG=debug rollping < hosts.txt

# Show only info and above (default)
rollping < hosts.txt

# Show only warnings and errors
RUST_LOG=warn rollping < hosts.txt

# Silent mode - only JSON output
RUST_LOG=error rollping < hosts.txt
```

## Geolocation

On first run, `rollping` automatically downloads the MaxMind GeoLite2-City database (~60MB) and caches it in `/tmp/rollping/`. This enables automatic geolocation of your current machine's public IP address.

The database is:
- Downloaded from a public GitHub mirror
- Cached for subsequent runs
- Stored in `/tmp/rollping/GeoLite2-City.mmdb`
- Works in restricted environments (e.g., cron jobs)

If geolocation fails or is unavailable, the tool continues normally without the `location` field in the output.

## Use Cases

### Network Monitoring

Monitor a list of critical servers and log results:

```bash
cat production-hosts.txt | RUST_LOG=error rollping >> latency-log.jsonl
```

### Cron Job

Add to crontab for periodic monitoring:

```bash
*/5 * * * * cat /path/to/hosts.txt | RUST_LOG=error rollping >> /var/www/html/latency.jsonl
```

### Integration with Monitoring Tools

```bash
# Send to monitoring endpoint
cat hosts.txt | RUST_LOG=error rollping | \
  curl -X POST -H "Content-Type: application/json" \
  -d @- https://monitoring.example.com/metrics
```

### Quick Network Health Check

```bash
# Check common DNS servers
echo -e "8.8.8.8\n8.8.4.4\n1.1.1.1\n1.0.0.1" | rollping | jq .avg_ms
```

## Requirements

- Rust 1.70 or later (for building from source)
- Network access for:
  - Pinging target hosts (ICMP)
  - Downloading GeoLite2 database (first run only)
  - Detecting public IP (if geolocation is enabled)

**Note:** On Linux and macOS, you may need elevated privileges to send ICMP packets. Run with `sudo` if you encounter permission errors.

## Performance

`rollping` is designed for efficiency:
- Concurrent pinging using Tokio async runtime
- Minimal memory footprint
- Fast startup time
- Efficient statistics calculation

Typical performance: ~100ms overhead for pinging 10 hosts with 3 pings each.

## License

MIT

## Contributing

Contributions welcome! Please feel free to submit a Pull Request.

## Repository

https://github.com/wbbradley/rollping
