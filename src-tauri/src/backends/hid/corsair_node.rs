//! Driver natif EXPÉRIMENTAL — Corsair Lighting Node Pro / Core.
//! Protocole documenté par le projet OpenRGB (CorsairLightingNodeController).
//! Écritures HID 64 octets, canaux R/G/B séparés par pages de 50 LEDs.
//! Activé uniquement si l'option "drivers natifs" est cochée.

use crate::core::Color;
use anyhow::{Context, Result};
use hidapi::HidDevice;

const PKT_SIZE: usize = 64;
const CMD_COMMIT: u8 = 0x33;
const CMD_BEGIN: u8 = 0x34;
const CMD_DIRECT: u8 = 0x32;
const CMD_PORT_STATE: u8 = 0x38;
const PORT_STATE_SOFTWARE: u8 = 0x02;
const LEDS_PER_PACKET: usize = 50;

pub struct CorsairLightingNode {
    device: HidDevice,
    /// Canaux déjà passés en contrôle logiciel.
    soft_mode: [bool; 2],
}

impl CorsairLightingNode {
    pub fn new(device: HidDevice) -> Self {
        CorsairLightingNode {
            device,
            soft_mode: [false; 2],
        }
    }

    fn write_packet(&self, payload: &[u8]) -> Result<()> {
        // Octet 0 = report ID 0x00, puis 64 octets de données.
        let mut buf = [0u8; PKT_SIZE + 1];
        let n = payload.len().min(PKT_SIZE);
        buf[1..1 + n].copy_from_slice(&payload[..n]);
        self.device
            .write(&buf)
            .context("écriture HID Corsair Lighting Node")?;
        Ok(())
    }

    fn ensure_software_mode(&mut self, channel: u8) -> Result<()> {
        let idx = (channel as usize).min(1);
        if !self.soft_mode[idx] {
            self.write_packet(&[CMD_PORT_STATE, channel, PORT_STATE_SOFTWARE])?;
            self.soft_mode[idx] = true;
        }
        Ok(())
    }

    /// Applique `colors` sur un canal (0 ou 1).
    pub fn set_channel_colors(&mut self, channel: u8, colors: &[Color]) -> Result<()> {
        self.ensure_software_mode(channel)?;
        self.write_packet(&[CMD_BEGIN, channel])?;

        // 3 passes : composante R (0), G (1), B (2), par pages de 50 LEDs.
        for (component, extract) in [
            (0u8, (|c: &Color| c.r) as fn(&Color) -> u8),
            (1u8, |c: &Color| c.g),
            (2u8, |c: &Color| c.b),
        ] {
            for (page, chunk) in colors.chunks(LEDS_PER_PACKET).enumerate() {
                let mut pkt = Vec::with_capacity(5 + chunk.len());
                pkt.extend_from_slice(&[
                    CMD_DIRECT,
                    channel,
                    (page * LEDS_PER_PACKET) as u8,
                    chunk.len() as u8,
                    component,
                ]);
                pkt.extend(chunk.iter().map(extract));
                self.write_packet(&pkt)?;
            }
        }
        self.write_packet(&[CMD_COMMIT, 0xFF])
    }
}
