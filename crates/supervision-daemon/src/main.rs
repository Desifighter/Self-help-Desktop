use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use clap::{Parser, Subcommand};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::sleep;
use tracing::{info, warn, error, Level};
use tracing_subscriber::FmtSubscriber;
use parking_lot::RwLock;

use supervision_core::classification::ClassificationCategory;
use supervision_core::config::{AppConfig, TamperGuard};
use supervision_core::dns::{DnsFilter, DnsProxy, HostsManager};
use supervision_core::monitor::{WindowMonitor, ProcessEviction};
use supervision_core::ipc::{IpcRequest, IpcResponse, DaemonStatus, BlockEvent, IPC_ADDR};

#[derive(Parser)]
#[command(name = "supervision-daemon")]
#[command(about = "Supervision Desktop Privileged Background Daemon", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run daemon in foreground (standard mode for dev/systemd/launchd)
    Run,
    /// Run as Windows Service
    #[cfg(windows)]
    Service,
}

fn get_config_path() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(program_data) = std::env::var("ProgramData") {
            return PathBuf::from(program_data).join("Supervision").join("config.json");
        }
    }
    #[cfg(not(windows))]
    {
        return PathBuf::from("/etc/supervision/config.json");
    }
    PathBuf::from("supervision_config.json")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => {
            info!("Starting Supervision Daemon in foreground mode...");
            run_daemon().await?;
        }
        #[cfg(windows)]
        Commands::Service => {
            info!("Starting Supervision Windows Service dispatcher...");
            run_windows_service()?;
        }
    }

    Ok(())
}

async fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path();
    let config = Arc::new(RwLock::new(AppConfig::load_or_default(&config_path)));
    let tamper = Arc::new(RwLock::new(TamperGuard::new(config.read().cooling_off_period_secs)));

    let filter = Arc::new(DnsFilter::new());
    *filter.block_adult.write() = config.read().filter_adult;
    *filter.block_social.write() = config.read().filter_social;
    *filter.block_gambling.write() = config.read().filter_gambling;

    for d in &config.read().custom_blocked_domains {
        filter.blocklist().add_custom_blocked(d);
    }
    for d in &config.read().custom_allowed_domains {
        filter.blocklist().add_safe_domain(d);
    }

    let listen_addr: SocketAddr = "127.0.0.1:53".parse()?;
    let upstream_addr: SocketAddr = "1.1.1.1:53".parse()?;
    let dns_proxy = Arc::new(DnsProxy::new(Arc::clone(&filter), listen_addr, upstream_addr));

    let start_time = Instant::now();
    let recent_events: Arc<RwLock<Vec<BlockEvent>>> = Arc::new(RwLock::new(Vec::new()));

    // 1. Spawn DNS Proxy Server
    {
        let proxy_clone = Arc::clone(&dns_proxy);
        tokio::spawn(async move {
            if let Err(e) = proxy_clone.run().await {
                warn!("Could not bind DNS port 53 directly (may require admin elevation): {}. Applying hosts table fallback...", e);
            }
        });
    }

    // 2. Synchronize hosts file as multi-layer defense
    {
        let hosts = HostsManager::new();
        let custom_list = config.read().custom_blocked_domains.clone();
        if let Err(e) = hosts.apply_blocklist(&custom_list) {
            warn!("Could not update hosts file: {}", e);
        }
    }

    // 3. Spawn Active Window & Process Monitor loop
    {
        let config_clone = Arc::clone(&config);
        let events_clone = Arc::clone(&recent_events);
        let heuristics = supervision_core::classification::HeuristicDomainClassifier::new();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_millis(500)).await;

                if let Some(win) = WindowMonitor::get_foreground_window() {
                    let is_restricted_app = config_clone.read().is_restricted_process(&win.process_name);
                    let is_adult_content = heuristics.classify_window_title(&win.window_title, &win.process_name);

                    if is_restricted_app || is_adult_content {
                        let category = if is_adult_content { "ADULT_CONTENT" } else { "APPLICATION" };
                        let action = if is_adult_content { "ACTIVE_WINDOW_EVICTED" } else { "EVICTED_AND_TERMINATED" };

                        info!("Intervention triggered: {} [{}] ({}) PID {}", win.process_name, win.window_title, category, win.process_id);

                        let event = BlockEvent {
                            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                            target: if is_adult_content { win.window_title.clone() } else { win.process_name.clone() },
                            category: category.to_string(),
                            action: action.to_string(),
                        };
                        {
                            let mut evs = events_clone.write();
                            evs.insert(0, event);
                            if evs.len() > 100 {
                                evs.truncate(100);
                            }
                        }

                        // Evict / minimize process
                        ProcessEviction::evict(win.process_id, &win.process_name);
                    }
                }
            }
        });
    }

    // 4. Start IPC Server for Desktop UI communication
    let ipc_listener = TcpListener::bind(IPC_ADDR).await?;
    info!("Supervision Daemon IPC listening on {}", IPC_ADDR);

    loop {
        let (mut socket, _) = match ipc_listener.accept().await {
            Ok(res) => res,
            Err(e) => {
                warn!("IPC accept error: {}", e);
                continue;
            }
        };

        let config_ref = Arc::clone(&config);
        let filter_ref = Arc::clone(&filter);
        let tamper_ref = Arc::clone(&tamper);
        let proxy_ref = Arc::clone(&dns_proxy);
        let events_ref = Arc::clone(&recent_events);
        let config_path_ref = config_path.clone();

        tokio::spawn(async move {
            let mut len_buf = [0u8; 4];
            if socket.read_exact(&mut len_buf).await.is_err() {
                return;
            }
            let len = u32::from_be_bytes(len_buf) as usize;
            if len > 65536 {
                return;
            }

            let mut req_buf = vec![0u8; len];
            if socket.read_exact(&mut req_buf).await.is_err() {
                return;
            }

            let request: IpcRequest = match serde_json::from_slice(&req_buf) {
                Ok(r) => r,
                Err(e) => {
                    let err_resp = IpcResponse::Error { error: format!("Invalid request: {}", e) };
                    let _ = send_ipc_response(&mut socket, &err_resp).await;
                    return;
                }
            };

            let response = handle_ipc_request(
                request,
                &config_ref,
                &filter_ref,
                &tamper_ref,
                &proxy_ref,
                &events_ref,
                &start_time,
                &config_path_ref,
            ).await;

            let _ = send_ipc_response(&mut socket, &response).await;
        });
    }
}

async fn handle_ipc_request(
    request: IpcRequest,
    config: &Arc<RwLock<AppConfig>>,
    filter: &Arc<DnsFilter>,
    tamper: &Arc<RwLock<TamperGuard>>,
    proxy: &Arc<DnsProxy>,
    events: &Arc<RwLock<Vec<BlockEvent>>>,
    start_time: &Instant,
    config_path: &PathBuf,
) -> IpcResponse {
    match request {
        IpcRequest::GetStatus => {
            let stats = proxy.stats().read();
            let cfg = config.read();
            let guard = tamper.read();

            let status = DaemonStatus {
                is_running: true,
                uptime_secs: start_time.elapsed().as_secs(),
                total_dns_queries: stats.total_queries,
                blocked_dns_queries: stats.blocked_queries,
                adult_blocked: stats.adult_blocked,
                social_blocked: stats.social_blocked,
                gambling_blocked: stats.gambling_blocked,
                custom_blocked: stats.custom_blocked,
                filter_adult: cfg.filter_adult,
                filter_social: cfg.filter_social,
                filter_gambling: cfg.filter_gambling,
                custom_blocked_domains: cfg.custom_blocked_domains.clone(),
                restricted_apps: cfg.restricted_applications.clone(),
                is_locked: !guard.can_unlock(),
                remaining_cooling_off_secs: guard.remaining_cooling_off_secs(),
            };
            IpcResponse::Status(status)
        }
        IpcRequest::UpdateFilterSettings { filter_adult, filter_social, filter_gambling } => {
            {
                let mut cfg = config.write();
                cfg.filter_adult = filter_adult;
                cfg.filter_social = filter_social;
                cfg.filter_gambling = filter_gambling;
                let _ = cfg.save(config_path);
            }
            *filter.block_adult.write() = filter_adult;
            *filter.block_social.write() = filter_social;
            *filter.block_gambling.write() = filter_gambling;

            IpcResponse::Success { message: "Filters updated successfully".to_string() }
        }
        IpcRequest::AddCustomBlockedDomain { domain } => {
            let clean = domain.trim().to_lowercase();
            {
                let mut cfg = config.write();
                if !cfg.custom_blocked_domains.contains(&clean) {
                    cfg.custom_blocked_domains.push(clean.clone());
                    let _ = cfg.save(config_path);
                }
            }
            filter.blocklist().add_custom_blocked(&clean);
            IpcResponse::Success { message: format!("Domain {} added to blocklist", clean) }
        }
        IpcRequest::RemoveCustomBlockedDomain { domain } => {
            let clean = domain.trim().to_lowercase();
            {
                let mut cfg = config.write();
                cfg.custom_blocked_domains.retain(|d| d != &clean);
                let _ = cfg.save(config_path);
            }
            filter.blocklist().remove_custom_blocked(&clean);
            IpcResponse::Success { message: format!("Domain {} removed from blocklist", clean) }
        }
        IpcRequest::AddRestrictedApp { app_name } => {
            let clean = app_name.trim().to_lowercase();
            {
                let mut cfg = config.write();
                if !cfg.restricted_applications.contains(&clean) {
                    cfg.restricted_applications.push(clean.clone());
                    let _ = cfg.save(config_path);
                }
            }
            IpcResponse::Success { message: format!("App {} added to restrictions", clean) }
        }
        IpcRequest::RemoveRestrictedApp { app_name } => {
            let clean = app_name.trim().to_lowercase();
            {
                let mut cfg = config.write();
                cfg.restricted_applications.retain(|a| a != &clean);
                let _ = cfg.save(config_path);
            }
            IpcResponse::Success { message: format!("App {} removed from restrictions", clean) }
        }
        IpcRequest::RequestUnlock => {
            let remaining = tamper.write().request_unlock();
            IpcResponse::Success {
                message: format!("Cooling-off timer started: {} seconds remaining before modifications allowed", remaining),
            }
        }
        IpcRequest::ConfirmUnlock { pin } => {
            let guard = tamper.read();
            if guard.can_unlock() {
                IpcResponse::Success { message: "Restrictions temporarily disarmed".to_string() }
            } else {
                IpcResponse::Error {
                    error: format!("Cooling-off period active. {} seconds remaining.", guard.remaining_cooling_off_secs()),
                }
            }
        }
        IpcRequest::GetRecentEvents => {
            let evs = events.read().clone();
            IpcResponse::RecentEvents(evs)
        }
    }
}

async fn send_ipc_response(socket: &mut tokio::net::TcpStream, resp: &IpcResponse) -> std::io::Result<()> {
    let payload = serde_json::to_vec(resp)?;
    let len_bytes = (payload.len() as u32).to_be_bytes();
    socket.write_all(&len_bytes).await?;
    socket.write_all(&payload).await?;
    Ok(())
}

#[cfg(windows)]
fn run_windows_service() -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::OsString;
    use windows_service::service_dispatcher;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};

    extern "system" fn service_main(_args: u32, _arg_ptrs: *mut *mut u16) {
        let event_handler = move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Stop | ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        };

        let status_handle = match service_control_handler::register("SupervisionService", event_handler) {
            Ok(h) => h,
            Err(_) => return,
        };

        let running_status = ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        };
        let _ = status_handle.set_service_status(running_status);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let _ = run_daemon().await;
        });

        let stopped_status = ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        };
        let _ = status_handle.set_service_status(stopped_status);
    }

    service_dispatcher::start("SupervisionService", service_main)?;
    Ok(())
}

