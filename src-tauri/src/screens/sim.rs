//! Écran simulé : accepte des images comme un vrai écran et garde la dernière.
//! Sert à essayer l'interface et le moteur sans matériel, et aux tests.

use super::driver::{Frame, Protocol, ScreenDriver, ScreenInfo, ScreenProvider};
use anyhow::Result;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct SimScreen {
    info: ScreenInfo,
    pub last: Arc<Mutex<Option<Frame>>>,
}

impl SimScreen {
    pub fn new(width: u32, height: u32) -> Self {
        SimScreen {
            info: ScreenInfo {
                id: "sim:0000:0000:1".into(),
                vendor: "PureRGB".into(),
                model: format!("Écran simulé {width}×{height}"),
                protocol: Protocol::Simulated,
                width,
                height,
                rotation: 0,
                resolution_confirmed: true,
                verified: true,
            },
            last: Arc::new(Mutex::new(None)),
        }
    }
}

impl ScreenDriver for SimScreen {
    fn info(&self) -> &ScreenInfo {
        &self.info
    }

    fn push_frame(&mut self, frame: &Frame) -> Result<()> {
        *self.last.lock() = Some(frame.clone());
        Ok(())
    }
}

/// Fournisseur d'un unique écran simulé (ajouté seulement à la demande).
pub struct SimProvider;

impl ScreenProvider for SimProvider {
    fn name(&self) -> &'static str {
        "simulé"
    }

    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>> {
        Ok(vec![Box::new(SimScreen::new(480, 480))])
    }
}
