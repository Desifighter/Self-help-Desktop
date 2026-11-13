use supervision_core::classification::{
    BlocklistProvider, HeuristicDomainClassifier, UpstreamCategorizationProvider,
};
use supervision_core::ipc::{IpcClient, IpcRequest, IpcResponse, DaemonStatus, BlockEvent};

#[tauri::command]
async fn get_daemon_status() -> Result<DaemonStatus, String> {
    match IpcClient::send_request(&IpcRequest::GetStatus).await {
        Ok(IpcResponse::Status(status)) => Ok(status),
        Ok(_) => Err("Invalid response from daemon".to_string()),
        Err(e) => Err(format!("Daemon not reachable: {}", e)),
    }
}

#[tauri::command]
async fn update_filters(filter_adult: bool, filter_social: bool, filter_gambling: bool) -> Result<String, String> {
    let req = IpcRequest::UpdateFilterSettings {
        filter_adult,
        filter_social,
        filter_gambling,
    };
    match IpcClient::send_request(&req).await {
        Ok(IpcResponse::Success { message }) => Ok(message),
        Ok(IpcResponse::Error { error }) => Err(error),
        _ => Err("Failed to update filters".to_string()),
    }
}

#[tauri::command]
async fn add_blocked_domain(domain: String) -> Result<String, String> {
    let req = IpcRequest::AddCustomBlockedDomain { domain };
    match IpcClient::send_request(&req).await {
        Ok(IpcResponse::Success { message }) => Ok(message),
        Ok(IpcResponse::Error { error }) => Err(error),
        _ => Err("Failed to add domain".to_string()),
    }
}

#[tauri::command]
async fn remove_blocked_domain(domain: String) -> Result<String, String> {
    let req = IpcRequest::RemoveCustomBlockedDomain { domain };
    match IpcClient::send_request(&req).await {
        Ok(IpcResponse::Success { message }) => Ok(message),
        Ok(IpcResponse::Error { error }) => Err(error),
        _ => Err("Failed to remove domain".to_string()),
    }
}

#[tauri::command]
async fn add_restricted_app(app_name: String) -> Result<String, String> {
    let req = IpcRequest::AddRestrictedApp { app_name };
    match IpcClient::send_request(&req).await {
        Ok(IpcResponse::Success { message }) => Ok(message),
        Ok(IpcResponse::Error { error }) => Err(error),
        _ => Err("Failed to add app".to_string()),
    }
}

#[tauri::command]
async fn remove_restricted_app(app_name: String) -> Result<String, String> {
    let req = IpcRequest::RemoveRestrictedApp { app_name };
    match IpcClient::send_request(&req).await {
        Ok(IpcResponse::Success { message }) => Ok(message),
        Ok(IpcResponse::Error { error }) => Err(error),
        _ => Err("Failed to remove app".to_string()),
    }
}

#[tauri::command]
async fn request_unlock() -> Result<String, String> {
    match IpcClient::send_request(&IpcRequest::RequestUnlock).await {
        Ok(IpcResponse::Success { message }) => Ok(message),
        Ok(IpcResponse::Error { error }) => Err(error),
        _ => Err("Failed to request unlock".to_string()),
    }
}

#[tauri::command]
async fn get_recent_events() -> Result<Vec<BlockEvent>, String> {
    match IpcClient::send_request(&IpcRequest::GetRecentEvents).await {
        Ok(IpcResponse::RecentEvents(events)) => Ok(events),
        _ => Ok(vec![]),
    }
}

#[tauri::command]
async fn test_domain(domain: String) -> Result<String, String> {
    let blocklist = BlocklistProvider::new();
    let heuristics = HeuristicDomainClassifier::new();
    let upstream = UpstreamCategorizationProvider::new();

    if blocklist.is_safe_allowlisted(&domain) {
        return Ok("ALLOWED: Essential safe allowlist override".to_string());
    }

    if let Some(c) = blocklist.check_domain(&domain, true, true, true) {
        return Ok(format!("BLOCKED: Found in {} blocklist (confidence: 1.0)", c.category));
    }

    if let Some(c) = heuristics.classify(&domain) {
        return Ok(format!("BLOCKED: Heuristic {} trigger (confidence: {:.2})", c.category, c.confidence));
    }

    if let Some(c) = upstream.query_categorization(&domain).await {
        return Ok(format!("BLOCKED: Upstream DNS privacy check ({})", c.category));
    }

    Ok("ALLOWED: Domain is safe and unrestricted".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_daemon_status,
            update_filters,
            add_blocked_domain,
            remove_blocked_domain,
            add_restricted_app,
            remove_restricted_app,
            request_unlock,
            get_recent_events,
            test_domain,
        ])
        .run(tauri::generate_context!())
        .expect("error while running supervision tauri application");
}
