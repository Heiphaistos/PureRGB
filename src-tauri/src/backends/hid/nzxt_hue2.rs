//! Driver natif EXPÉRIMENTAL — NZXT HUE 2 / Smart Device V2 / RGB & Fan Controller.
//! Protocole documenté par le projet liquidctl (hue2.py, smart_device.py).
//! Écritures HID 64 octets. LEDs en mode direct ("super-fixed") + PWM ventilateurs.
//! Activé uniquement si l'option "drivers natifs" est cochée.

use crate::core::Color;
use anyhow::{Context, Result};
use hidapi::HidDevice;

const PKT_SIZE: usize = 64;
const LEDS_PER_PACKET: usize = 20;

pub struct NzxtHue2 {
    device: HidDevice,
}

impl NzxtHue2 {
    pub fn new(device: HidDevice) -> Self {
        NzxtHue2 { device }
    }

    fn write_packet(&self, payload: &[u8]) -> Result<()> {
        // Octet 0 = report ID 0x00 (rapports non numérotés), puis 64 octets.
        let mut buf = [0u8; PKT_SIZE + 1];
        let n = payload.len().min(PKT_SIZE);
        buf[1..1 + n].copy_from_slice(&payload[..n]);
        self.device.write(&buf).context("écriture HID NZXT HUE2")?;
        Ok(())
    }

    /// Mode direct : pages de 20 LEDs [0x22, 0x10 + page, canal] + apply.
    /// `channel` est 1-indexé côté protocole (canal LED 1 = 0x01).
    pub fn set_channel_colors(&mut self, channel: u8, colors: &[Color]) -> Result<()> {
        let cid = channel + 1;
        for (page, chunk) in colors.chunks(LEDS_PER_PACKET).enumerate() {
            let mut pkt = Vec::with_capacity(4 + chunk.len() * 3);
            pkt.extend_from_slice(&[0x22, 0x10 + page as u8, cid, 0x00]);
            for c in chunk {
                // Ordre GRB sur le bus HUE2.
                pkt.extend_from_slice(&[c.g, c.r, c.b]);
            }
            self.write_packet(&pkt)?;
        }
        // Apply : mode "super-fixed" (0x00), pas de vitesse.
        self.write_packet(&[
            0x22, 0xA0, cid, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x80, 0x00, 0x32, 0x00, 0x00, 0x01,
        ])
    }

    /// PWM ventilateur : canal 0-2, duty 0-100 %.
    pub fn set_fan_duty(&mut self, channel: u8, percent: u8) -> Result<()> {
        let ch = channel.min(2);
        let mut pkt = [0u8; 8];
        pkt[0] = 0x62;
        pkt[1] = 0x01;
        pkt[2] = 0x01 << ch;
        pkt[3 + ch as usize] = percent.min(100);
        self.write_packet(&pkt)
    }
}
