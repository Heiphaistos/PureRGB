//! Contrat commun des pilotes d'écrans : un pilote ne connaît que `Frame` (RGBA)
//! et le convertit vers le format de fil de son matériel.

use super::drivers::wire::WireImage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Image prête à envoyer : RGBA 8 bits, ligne par ligne, sans padding.
#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Frame {
    pub fn new(width: u32, height: u32) -> Self {
        Frame { width, height, rgba: vec![0; (width as usize) * (height as usize) * 4] }
    }

    pub fn is_valid(&self) -> bool {
        self.rgba.len() == (self.width as usize) * (self.height as usize) * 4
    }
}

/// Famille de protocole (sert à l'affichage et au diagnostic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    ThermalrightScsi,
    ThermalrightHid,
    ThermalrightBulk,
    ThermalrightLy,
    TuringSerial,
    CorsairLcd,
    DeepcoolLm,
    AsusRyujin,
    ThermaltakeLcd,
    LianLiHid,
    TurzxUsb,
    Simulated,
}

/// Fiche d'un écran détecté, envoyée telle quelle à l'interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenInfo {
    /// Identifiant stable : `<vid>:<pid>:<serie|chemin>`.
    pub id: String,
    pub vendor: String,
    pub model: String,
    pub protocol: Protocol,
    pub width: u32,
    pub height: u32,
    /// Rotation matérielle à appliquer avant envoi (0, 90, 180, 270).
    pub rotation: u16,
    /// Vrai si la résolution a été lue sur l'appareil, faux si déduite du catalogue.
    pub resolution_confirmed: bool,
    /// Matériel validé par un test réel (sinon « expérimental » dans l'interface).
    pub verified: bool,
}

/// Encodage d'une image hors du thread d'envoi (pilotes JPEG Thermalright) : permet de
/// recouvrir le calcul d'une image et le transfert USB de la précédente.
pub trait FramePreparer: Send + Sync {
    /// Encode `frame` ; `max_quality` plafonne la qualité JPEG de départ du pilote.
    fn prepare(&self, frame: &Frame, max_quality: u8) -> Result<WireImage>;
}

/// Un écran ouvert, prêt à recevoir des images.
pub trait ScreenDriver: Send {
    fn info(&self) -> &ScreenInfo;

    /// Cadence maximale utile du matériel (i/s) ; le planificateur ne la dépasse pas.
    fn max_fps(&self) -> u8 {
        30
    }

    /// Encodeur autonome si le pilote sait préparer ses images hors du thread d'envoi ;
    /// l'envoi passe alors par `send_prepared`. Défaut : aucun (`push_frame` fait tout).
    fn preparer(&self) -> Option<Arc<dyn FramePreparer>> {
        None
    }

    /// Envoie une image produite par `preparer().prepare(..)`.
    fn send_prepared(&mut self, _img: &WireImage) -> Result<()> {
        anyhow::bail!("cet écran n'a pas de pré-encodage")
    }

    /// Envoie une image. `frame` a exactement la taille `info().width × height`.
    fn push_frame(&mut self, frame: &Frame) -> Result<()>;

    /// Luminosité matérielle 0-100 si le protocole la supporte ; sinon
    /// `Ok(false)` (la luminosité est alors appliquée au rendu côté logiciel).
    fn set_brightness(&mut self, _percent: u8) -> Result<bool> {
        Ok(false)
    }

    /// Faux si l'écran n'accepte pas de flux d'images en direct (ex. AIO dont seul
    /// le stockage d'une image fixe est connu) : le gestionnaire n'envoie alors rien en boucle.
    fn live_frames(&self) -> bool {
        true
    }

    /// Envoie UNE image fixe mémorisée par l'écran (écriture flash : usage ponctuel).
    fn upload_still(&mut self, _frame: &Frame) -> Result<()> {
        anyhow::bail!("cet écran ne sait pas mémoriser une image fixe")
    }
}

/// Un catalogue de détection : énumère les écrans présents et les ouvre.
pub trait ScreenProvider: Send {
    fn name(&self) -> &'static str;
    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>>;
}
