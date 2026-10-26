//! Transmission's own settings (Settings › Transmission), kept on the host so they
//! apply whichever computer changes them, and the turtle (alternative speeds).

use tauri::State;
use transmission::{PortTest, Session, SessionSet};

use crate::commands::Problem;
use crate::poller::Poller;

fn client(poller: &Poller) -> Result<std::sync::Arc<transmission::Client>, Problem> {
    poller.client().ok_or_else(|| Problem::from("Not connected to Transmission.".to_owned()))
}

/// Every setting.
#[tauri::command]
pub async fn session_settings(poller: State<'_, Poller>) -> Result<Session, Problem> {
    client(&poller)?.session_get(&[]).await.map_err(|e| Problem::from(&e))
}

/// Change settings, and answer with all of them as they are now: one change can
/// move others (turning a schedule on switches the turtle by the clock).
#[tauri::command]
pub async fn set_session(set: SessionSet, poller: State<'_, Poller>) -> Result<Session, Problem> {
    let client = client(&poller)?;
    let result = client.session_set(&set).await;
    poller.poll_now();
    result.map_err(|e| Problem::from(&e))?;
    client.session_get(&[]).await.map_err(|e| Problem::from(&e))
}

/// The turtle and speed limits as of the last poll (then `session` events).
#[tauri::command]
pub fn session_brief(poller: State<'_, Poller>) -> Option<Session> {
    poller.session()
}

/// Whether the peer port can be reached from the internet.
#[tauri::command]
pub async fn test_port(poller: State<'_, Poller>) -> Result<PortTest, Problem> {
    client(&poller)?.port_test(false).await.map_err(|e| Problem::from(&e))
}

/// Fetch the blocklist from its URL now; how many rules it has.
#[tauri::command]
pub async fn update_blocklist(poller: State<'_, Poller>) -> Result<i64, Problem> {
    client(&poller)?.blocklist_update().await.map_err(|e| Problem::from(&e))
}
