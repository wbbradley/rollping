use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead};
use std::net::ToSocketAddrs;
use std::time::Duration;
use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

#[derive(Parser, Debug)]
#[command(name = "rollping")]
#[command(about = "Ping multiple hosts and aggregate statistics", long_about = None)]
struct Args {
    /// Number of pings to send to each host
    #[arg(short = 'c', long, default_value = "3")]
    count: usize,

    /// Timeout in seconds for each ping
    #[arg(short = 't', long, default_value = "2.0")]
    timeout_secs: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Statistics {
    /// Average ping time in milliseconds
    avg_ms: f64,
    /// Median ping time in milliseconds
    median_ms: f64,
    /// 95th percentile ping time in milliseconds
    p95_ms: f64,
    /// 99th percentile ping time in milliseconds
    p99_ms: f64,
    /// Maximum ping time in milliseconds
    max_ms: f64,
    /// Number of hosts that failed to respond
    non_responsive_nodes: usize,
    /// Total number of hosts tested
    total_hosts: usize,
    /// Number of pings sent to each host
    pings_per_host: usize,
    /// Timeout in seconds for each ping
    timeout_secs: f64,
}

#[derive(Debug)]
struct HostResult {
    host: String,
    best_time_ms: Option<f64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing to stderr with RUST_LOG support
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let args = Args::parse();
    info!("Starting rollping with {} pings per host, {}s timeout", args.count, args.timeout_secs);

    // Read hosts from stdin
    let hosts = read_hosts_from_stdin()?;
    info!("Read {} hosts from stdin", hosts.len());

    if hosts.is_empty() {
        warn!("No hosts provided on stdin");
        let stats = Statistics {
            avg_ms: 0.0,
            median_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            max_ms: 0.0,
            non_responsive_nodes: 0,
            total_hosts: 0,
            pings_per_host: args.count,
            timeout_secs: args.timeout_secs,
        };
        println!("{}", serde_json::to_string(&stats)?);
        return Ok(());
    }

    // Ping all hosts concurrently
    let timeout_duration = Duration::from_secs_f64(args.timeout_secs);
    let results = ping_hosts(&hosts, args.count, timeout_duration).await;

    // Calculate statistics
    let stats = calculate_statistics(&results, args.count, args.timeout_secs);
    info!("Completed pinging {} hosts, {} non-responsive", stats.total_hosts, stats.non_responsive_nodes);

    // Output JSON to stdout
    println!("{}", serde_json::to_string(&stats)?);

    Ok(())
}

fn read_hosts_from_stdin() -> Result<Vec<String>> {
    let stdin = io::stdin();
    let hosts: Vec<String> = stdin
        .lock()
        .lines()
        .filter_map(|line| {
            line.ok()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
        })
        .collect();
    Ok(hosts)
}

async fn ping_hosts(hosts: &[String], count: usize, timeout_duration: Duration) -> Vec<HostResult> {
    let mut handles = Vec::new();

    for host in hosts {
        let host = host.clone();
        let handle = tokio::spawn(async move {
            ping_host(&host, count, timeout_duration).await
        });
        handles.push(handle);
    }

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(result) => results.push(result),
            Err(e) => {
                error!("Task join error: {}", e);
            }
        }
    }

    results
}

async fn ping_host(host: &str, count: usize, timeout_duration: Duration) -> HostResult {
    debug!("Pinging host: {} ({} times)", host, count);

    let config = Config::default();
    let client = match Client::new(&config) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to create ping client for {}: {}", host, e);
            return HostResult {
                host: host.to_string(),
                best_time_ms: None,
            };
        }
    };

    let mut min_time_ms: Option<f64> = None;
    let mut successful_pings = 0;

    for i in 0..count {
        match timeout(
            timeout_duration,
            ping_once(&client, host, i as u16)
        ).await {
            Ok(Ok(rtt)) => {
                let rtt_ms = rtt.as_secs_f64() * 1000.0;
                debug!("Host {} ping #{}: {:.2}ms", host, i + 1, rtt_ms);
                min_time_ms = Some(min_time_ms.map_or(rtt_ms, |min| min.min(rtt_ms)));
                successful_pings += 1;
            }
            Ok(Err(e)) => {
                warn!("Host {} ping #{} failed: {}", host, i + 1, e);
            }
            Err(_) => {
                warn!("Host {} ping #{} timed out", host, i + 1);
            }
        }
    }

    if let Some(best) = min_time_ms {
        info!("Host {} best time: {:.2}ms ({}/{} successful)", host, best, successful_pings, count);
    } else {
        warn!("Host {} failed all pings", host);
    }

    HostResult {
        host: host.to_string(),
        best_time_ms: min_time_ms,
    }
}

async fn ping_once(client: &Client, host: &str, seq: u16) -> Result<Duration> {
    // Resolve hostname to IP address
    let ip_addr = format!("{}:0", host)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve host: {}", host))?
        .ip();

    let mut pinger = client.pinger(ip_addr, PingIdentifier(rand::random())).await;

    let payload = [0; 8];
    let (_packet, duration) = pinger.ping(PingSequence(seq), &payload).await
        .map_err(|e| anyhow::anyhow!("Ping failed: {}", e))?;

    Ok(duration)
}

fn calculate_statistics(results: &[HostResult], pings_per_host: usize, timeout_secs: f64) -> Statistics {
    let mut successful_times: Vec<f64> = results
        .iter()
        .filter_map(|r| r.best_time_ms)
        .collect();

    let non_responsive_nodes = results.iter().filter(|r| r.best_time_ms.is_none()).count();
    let total_hosts = results.len();

    if successful_times.is_empty() {
        return Statistics {
            avg_ms: 0.0,
            median_ms: 0.0,
            p95_ms: 0.0,
            p99_ms: 0.0,
            max_ms: 0.0,
            non_responsive_nodes,
            total_hosts,
            pings_per_host,
            timeout_secs,
        };
    }

    successful_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let avg_ms = successful_times.iter().sum::<f64>() / successful_times.len() as f64;
    let median_ms = percentile(&successful_times, 50.0);
    let p95_ms = percentile(&successful_times, 95.0);
    let p99_ms = percentile(&successful_times, 99.0);
    let max_ms = *successful_times.last().unwrap();

    Statistics {
        avg_ms,
        median_ms,
        p95_ms,
        p99_ms,
        max_ms,
        non_responsive_nodes,
        total_hosts,
        pings_per_host,
        timeout_secs,
    }
}

fn percentile(sorted_values: &[f64], p: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }

    let idx = (p / 100.0 * (sorted_values.len() - 1) as f64).round() as usize;
    sorted_values[idx.min(sorted_values.len() - 1)]
}
