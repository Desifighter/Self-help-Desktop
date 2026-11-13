use clap::{Parser, Subcommand};
use supervision_core::classification::{
    BlocklistProvider, HeuristicDomainClassifier, UpstreamCategorizationProvider,
};
use supervision_core::ipc::{IpcClient, IpcRequest, IpcResponse};

#[derive(Parser)]
#[command(name = "supervision-cli")]
#[command(about = "Supervision Desktop Administrative CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get live daemon status and blocking statistics
    Status,
    /// Test domain classification locally
    TestDomain {
        /// Domain to evaluate (e.g. pornhub.com, github.com)
        domain: String,
    },
    /// Add custom domain to blocklist
    BlockDomain {
        domain: String,
    },
    /// Remove domain from blocklist
    UnblockDomain {
        domain: String,
    },
    /// Add restricted application executable name
    BlockApp {
        app_name: String,
    },
    /// Remove application from restrictions
    UnblockApp {
        app_name: String,
    },
    /// Request disarming / cooling-off countdown
    RequestUnlock,
    /// View recent block events
    Logs,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Status => {
            println!("Connecting to Supervision Daemon via IPC...");
            match IpcClient::send_request(&IpcRequest::GetStatus).await {
                Ok(IpcResponse::Status(status)) => {
                    println!("\n=== Supervision Desktop Daemon Status ===");
                    println!("Daemon Running:            {}", if status.is_running { "YES" } else { "NO" });
                    println!("Uptime:                    {} seconds", status.uptime_secs);
                    println!("Total DNS Queries:         {}", status.total_dns_queries);
                    println!("Blocked DNS Queries:       {}", status.blocked_dns_queries);
                    println!("  ├─ Adult Content:        {}", status.adult_blocked);
                    println!("  ├─ Social Media:         {}", status.social_blocked);
                    println!("  ├─ Gambling:             {}", status.gambling_blocked);
                    println!("  └─ Custom Blocked:       {}", status.custom_blocked);
                    println!("\nActive Filter Categories:");
                    println!("  ├─ Adult Content:        {}", if status.filter_adult { "ENABLED" } else { "DISABLED" });
                    println!("  ├─ Social Media:         {}", if status.filter_social { "ENABLED" } else { "DISABLED" });
                    println!("  └─ Gambling:             {}", if status.filter_gambling { "ENABLED" } else { "DISABLED" });
                    println!("\nRestricted Apps:           {:?}", status.restricted_apps);
                    println!("Custom Blocked Domains:    {:?}", status.custom_blocked_domains);
                    println!("Tamper Lock Active:        {}", if status.is_locked { "LOCKED (Cooling-off active)" } else { "UNLOCKED" });
                    if status.is_locked {
                        println!("Cooling-Off Remaining:     {} seconds", status.remaining_cooling_off_secs);
                    }
                }
                Ok(other) => println!("Unexpected response: {:?}", other),
                Err(e) => {
                    eprintln!("Error connecting to daemon: {}. Is supervision-daemon running?", e);
                }
            }
        }
        Commands::TestDomain { domain } => {
            println!("Evaluating domain '{}' across all classification engines...", domain);
            let blocklist = BlocklistProvider::new();
            let heuristics = HeuristicDomainClassifier::new();
            let upstream = UpstreamCategorizationProvider::new();

            // 1. Safe allowlist
            if blocklist.is_safe_allowlisted(&domain) {
                println!("Result: ALLOWED (Essential safe allowlist override)");
                return Ok(());
            }

            // 2. Blocklist
            if let Some(c) = blocklist.check_domain(&domain, true, true, true) {
                println!("Result: BLOCKED");
                println!("  Category:   {}", c.category);
                println!("  Source:     {:?}", c.source);
                println!("  Confidence: {:.2}", c.confidence);
                return Ok(());
            }

            // 3. Heuristic
            if let Some(c) = heuristics.classify(&domain) {
                println!("Result: BLOCKED");
                println!("  Category:   {}", c.category);
                println!("  Source:     {:?}", c.source);
                println!("  Confidence: {:.2}", c.confidence);
                return Ok(());
            }

            // 4. Upstream query
            print!("Checking upstream privacy DNS (Cloudflare Families / CleanBrowsing)... ");
            if let Some(c) = upstream.query_categorization(&domain).await {
                println!("BLOCKED");
                println!("  Category:   {}", c.category);
                println!("  Source:     {:?}", c.source);
                println!("  Confidence: {:.2}", c.confidence);
                return Ok(());
            }

            println!("ALLOWED (Domain appears safe or unclassified)");
        }
        Commands::BlockDomain { domain } => {
            match IpcClient::send_request(&IpcRequest::AddCustomBlockedDomain { domain: domain.clone() }).await {
                Ok(IpcResponse::Success { message }) => println!("Success: {}", message),
                Ok(IpcResponse::Error { error }) => eprintln!("Error: {}", error),
                _ => eprintln!("Failed to update blocklist"),
            }
        }
        Commands::UnblockDomain { domain } => {
            match IpcClient::send_request(&IpcRequest::RemoveCustomBlockedDomain { domain }).await {
                Ok(IpcResponse::Success { message }) => println!("Success: {}", message),
                Ok(IpcResponse::Error { error }) => eprintln!("Error: {}", error),
                _ => eprintln!("Failed to remove domain"),
            }
        }
        Commands::BlockApp { app_name } => {
            match IpcClient::send_request(&IpcRequest::AddRestrictedApp { app_name }).await {
                Ok(IpcResponse::Success { message }) => println!("Success: {}", message),
                Ok(IpcResponse::Error { error }) => eprintln!("Error: {}", error),
                _ => eprintln!("Failed to add restricted app"),
            }
        }
        Commands::UnblockApp { app_name } => {
            match IpcClient::send_request(&IpcRequest::RemoveRestrictedApp { app_name }).await {
                Ok(IpcResponse::Success { message }) => println!("Success: {}", message),
                Ok(IpcResponse::Error { error }) => eprintln!("Error: {}", error),
                _ => eprintln!("Failed to remove restricted app"),
            }
        }
        Commands::RequestUnlock => {
            match IpcClient::send_request(&IpcRequest::RequestUnlock).await {
                Ok(IpcResponse::Success { message }) => println!("{}", message),
                Ok(IpcResponse::Error { error }) => eprintln!("Error: {}", error),
                _ => eprintln!("Failed to request unlock"),
            }
        }
        Commands::Logs => {
            match IpcClient::send_request(&IpcRequest::GetRecentEvents).await {
                Ok(IpcResponse::RecentEvents(events)) => {
                    println!("\n=== Recent Supervision Interventions ===");
                    if events.is_empty() {
                        println!("No interventions recorded yet.");
                    } else {
                        for ev in events {
                            println!("[{}] {} -> Target: {} ({})", ev.timestamp, ev.action, ev.target, ev.category);
                        }
                    }
                }
                _ => eprintln!("Failed to fetch logs"),
            }
        }
    }

    Ok(())
}
