# rollping

A fast, concurrent ping statistics aggregator with built-in geolocation support.

`rollping` reads a list of hosts from stdin, pings them concurrently, and outputs aggregated statistics as single-line JSON. Perfect for monitoring network latency across multiple hosts and integrating with other command-line tools.

## Features

- **Concurrent pinging** - Tests multiple hosts simultaneously for fast results
- **Statistical analysis** - Calculates avg, median, p95, p99, and max latency
- **Optional geolocation** - Opt-in location detection using MaxMind GeoLite2 (use `-g` flag)
- **Composable output** - Single-line JSON output, silent by default for easy piping
- **Configurable logging** - Use `-v` for warnings, `-vv` for info, or `RUST_LOG` environment variable
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

### Basic Example (Silent by default)

```bash
echo -e "8.8.8.8\n1.1.1.1\n1.0.0.1" | rollping
```

Output is JSON only with no log messages.

### With Verbose Logging

```bash
# Show warnings (-v)
echo -e "8.8.8.8\n1.1.1.1" | rollping -v

# Show info messages (-vv)
echo -e "8.8.8.8\n1.1.1.1" | rollping -vv
```

### With Geolocation

```bash
# Include location data in output
echo -e "8.8.8.8\n1.1.1.1" | rollping -g

# With geolocation and verbose logging
echo -e "8.8.8.8\n1.1.1.1" | rollping -g -vv
```

### Custom Settings

```bash
# Send 5 pings to each host with 1 second timeout
echo -e "8.8.8.8\n1.1.1.1" | rollping -c 5 -t 1.0
```

### From a File

```bash
cat hosts.txt | rollping -c 3
```

### Pretty Print with jq

```bash
echo -e "8.8.8.8\n1.1.1.1" | rollping | jq .
```

## Output Format

The output is a single-line JSON object with the following fields:

```json
{
  "timestamp": 1763421627,
  "avg_microsecs": 4235,
  "median_microsecs": 4567,
  "p95_microsecs": 5123,
  "p99_microsecs": 5234,
  "max_microsecs": 5345,
  "non_responsive_nodes": 0,
  "total_hosts": 2,
  "pings_per_host": 3,
  "timeout_secs": 2.0
}
```

**Field descriptions:**
- `timestamp`: Unix epoch timestamp (seconds) when the measurement was taken
- `*_microsecs` fields: All latency values in microseconds, rounded to the nearest integer

**Note:** The `location` field is only included when using the `-g/--geo` flag:

```json
{
  ...
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

  -v, --verbose
          Increase logging verbosity (-v for WARN, -vv for INFO)

  -g, --geo
          Enable geolocation (fetches and includes location data)

  -h, --help
          Print help

  -V, --version
          Print version
```

## Logging

By default, `rollping` only shows errors. Use the `-v` flag for more verbosity:

```bash
# Silent mode - only errors and JSON output (default)
rollping < hosts.txt

# Show warnings (-v)
rollping -v < hosts.txt

# Show info messages (-vv)
rollping -vv < hosts.txt
```

You can also use the `RUST_LOG` environment variable to override the log level:

```bash
# Show debug logs
RUST_LOG=debug rollping < hosts.txt

# Show only errors (same as default)
RUST_LOG=error rollping < hosts.txt
```

## Geolocation

Geolocation is **opt-in** and disabled by default. Use the `-g/--geo` flag to enable it:

```bash
rollping -g < hosts.txt
```

When enabled, `rollping` downloads the MaxMind GeoLite2-City database (~60MB) on first run and caches it in `/tmp/rollping/`. This enables geolocation of your current machine's public IP address.

The database is:
- Downloaded from a public GitHub mirror (first use only)
- Cached for subsequent runs
- Stored in `/tmp/rollping/GeoLite2-City.mmdb`
- Works in restricted environments (e.g., cron jobs)

If geolocation fails or is unavailable, the tool continues normally without the `location` field in the output.

## Use Cases

### Network Monitoring

Monitor a list of critical servers and log results:

```bash
cat production-hosts.txt | rollping >> latency-log.jsonl
```

### Cron Job

Add to crontab for periodic monitoring:

```bash
*/5 * * * * cat /path/to/hosts.txt | rollping >> /var/www/html/latency.jsonl
```

### Integration with Monitoring Tools

```bash
# Send to monitoring endpoint
cat hosts.txt | rollping | \
  curl -X POST -H "Content-Type: application/json" \
  -d @- https://monitoring.example.com/metrics
```

### With Geolocation for Location Tracking

```bash
# Track your location along with latency data
cat hosts.txt | rollping -g >> latency-log.jsonl
```

### Quick Network Health Check

```bash
# Check common DNS servers (output in microseconds)
echo -e "8.8.8.8\n8.8.4.4\n1.1.1.1\n1.0.0.1" | rollping | jq .avg_microsecs
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
