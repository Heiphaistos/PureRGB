//! Contrat commun des pilotes d'écrans : un pilote ne connaît que `Frame` (RGBA)
//! et le convertit vers le format de fil de son matériel.

use anyhow::Result;
use serde::{Deserialize, Serialize};

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

/// Un écran ouvert, prêt à recevoir des images.
pub trait ScreenDriver: Send {
    fn info(&self) -> &ScreenInfo;

    /// Envoie une image. `frame` a exactement la taille `info().width × height`.
    fn push_frame(&mut self, frame: &Frame) -> Result<()>;

    /// Luminosité matérielle 0-100 si le protocole la supporte ; sinon
    /// `Ok(false)` (la luminosité est alors appliquée au rendu côté logiciel).
    fn set_brightness(&mut self, _percent: u8) -> Result<bool> {
        Ok(false)
    }
}

/// Un catalogue de détection : énumère les écrans présents et les ouvre.
pub trait ScreenProvider: Send {
    fn name(&self) -> &'static str;
    fn detect(&mut self) -> Result<Vec<Box<dyn ScreenDriver>>>;
}
