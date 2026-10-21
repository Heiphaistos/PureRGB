//! Fermeture de l'application : jamais sur le thread de l'interface (une étape lente
//! donnait « ne répond plus »), toujours bornée dans le temps, et garantie de sortie
//! du processus même si un pilote USB ou un logiciel tiers est bloqué.

use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Manager};

/// Délai laissé au nettoyage (restauration des logiciels constructeur, arrêt des moteurs…).
const CLEANUP_DEADLINE: Duration = Duration::from_secs(8);
/// Délai entre `app.exit` et la sortie forcée du processus.
const FORCE_EXIT_AFTER: Duration = Duration::from_secs(3);

/// Lance la fermeture : cache les fenêtres tout de suite, nettoie en arrière-plan, puis quitte.
/// Rend la main immédiatement (le thread d'interface reste réactif).
pub fn quit_async(app: AppHandle, cleanup: fn(&AppHandle)) {
    for (_, w) in app.webview_windows() {
        let _ = w.hide();
    }
    let spawned = std::thread::Builder::new().name("quit".into()).spawn(move || {
        run_bounded(&app, cleanup, CLEANUP_DEADLINE);
        // Filet : si la boucle d'événements ne se termine pas, on sort quand même.
        let _ = std::thread::Builder::new().name("force-exit".into()).spawn(|| {
            std::thread::sleep(FORCE_EXIT_AFTER);
            log::warn!("sortie forcée du processus");
            std::process::exit(0);
        });
        app.exit(0);
    });
    if spawned.is_err() {
        // Impossible de créer le thread : sortie directe (les enfants meurent avec le job Windows).
        std::process::exit(0);
    }
}

/// Exécute `cleanup` dans un thread et attend au plus `deadline` ; au-delà, on continue sans lui.
fn run_bounded(app: &AppHandle, cleanup: fn(&AppHandle), deadline: Duration) {
    let (tx, rx) = mpsc::channel();
    let handle = app.clone();
    let started = std::thread::Builder::new().name("cleanup".into()).spawn(move || {
        cleanup(&handle);
        let _ = tx.send(());
    });
    match started {
        Ok(_) => {
            if rx.recv_timeout(deadline).is_err() {
                log::warn!("nettoyage de fermeture trop long (> {deadline:?}) : abandon");
            }
        }
        Err(e) => log::warn!("thread de nettoyage: {e}"),
    }
}

/// Test d'acceptation automatisé : `PURERGB_AUTOQUIT_SECS=<n>` ferme l'application proprement
/// après `n` secondes (chemin identique au menu « Quitter »), pour vérifier qu'aucun processus ne reste.
pub fn schedule_autoquit(app: &AppHandle, cleanup: fn(&AppHandle)) {
    let Some(secs) = std::env::var("PURERGB_AUTOQUIT_SECS").ok().and_then(|v| v.parse::<u64>().ok()) else {
        return;
    };
    let app = app.clone();
    let _ = std::thread::Builder::new().name("autoquit".into()).spawn(move || {
        std::thread::sleep(Duration::from_secs(secs.min(600)));
        log::info!("fermeture automatique (PURERGB_AUTOQUIT_SECS)");
        quit_async(app, cleanup);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_wait_gives_up_on_a_stuck_cleanup() {
        let (tx, rx) = mpsc::channel::<()>();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(5));
            let _ = tx.send(());
        });
        let t = std::time::Instant::now();
        assert!(rx.recv_timeout(Duration::from_millis(200)).is_err());
        assert!(t.elapsed() < Duration::from_secs(2));
    }
}
