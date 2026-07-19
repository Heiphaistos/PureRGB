//! Protocole binaire OpenRGB SDK (little-endian).
//! On négocie la version 1 du protocole : suffisante pour lire les
//! contrôleurs (nom, vendor, type, zones, leds) et écrire les couleurs,
//! sans les champs brightness/segments des versions 3+.

use crate::core::{DeviceType, ModeInfo, ZoneInfo};
use anyhow::{bail, Context, Result};

pub const MAGIC: &[u8; 4] = b"ORGB";
pub const PROTOCOL_VERSION: u32 = 1;

pub const REQUEST_CONTROLLER_COUNT: u32 = 0;
pub const REQUEST_CONTROLLER_DATA: u32 = 1;
pub const REQUEST_PROTOCOL_VERSION: u32 = 40;
pub const SET_CLIENT_NAME: u32 = 50;
pub const RGBCONTROLLER_UPDATELEDS: u32 = 1050;
pub const RGBCONTROLLER_SETCUSTOMMODE: u32 = 1100;
pub const RGBCONTROLLER_UPDATEMODE: u32 = 1101;

/// En-tête de paquet : magic + device_id + packet_id + taille données.
pub fn header(device_id: u32, packet_id: u32, data_len: u32) -> [u8; 16] {
    let mut h = [0u8; 16];
    h[0..4].copy_from_slice(MAGIC);
    h[4..8].copy_from_slice(&device_id.to_le_bytes());
    h[8..12].copy_from_slice(&packet_id.to_le_bytes());
    h[12..16].copy_from_slice(&data_len.to_le_bytes());
    h
}

/// Payload UpdateLeds : u32 taille totale, u16 nb couleurs, puis u32 par LED
/// (0x00BBGGRR little-endian => octets R, G, B, 0).
pub fn encode_update_leds(colors: &[crate::core::Color]) -> Vec<u8> {
    let mut data = Vec::with_capacity(6 + colors.len() * 4);
    let total = (6 + colors.len() * 4) as u32;
    data.extend_from_slice(&total.to_le_bytes());
    data.extend_from_slice(&(colors.len() as u16).to_le_bytes());
    for c in colors {
        data.extend_from_slice(&[c.r, c.g, c.b, 0]);
    }
    data
}

/// Contrôleur tel que décrit par le serveur.
#[derive(Debug, Clone)]
pub struct ControllerData {
    pub device_type: DeviceType,
    pub name: String,
    pub vendor: String,
    pub zones: Vec<ZoneInfo>,
    pub led_count: u32,
    pub modes: Vec<ModeInfo>,
    pub active_mode: i32,
}

/// Payload UpdateMode (v1) : taille totale, id du mode, puis le mode complet
/// dans le même format que controller_data.
pub fn encode_update_mode(mode: &ModeInfo) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::new();
    body.extend_from_slice(&(mode.index as i32).to_le_bytes());
    let name_len = (mode.name.len() + 1) as u16;
    body.extend_from_slice(&name_len.to_le_bytes());
    body.extend_from_slice(mode.name.as_bytes());
    body.push(0);
    body.extend_from_slice(&mode.value.to_le_bytes());
    for v in [
        mode.flags,
        mode.speed_min,
        mode.speed_max,
        mode.colors_min,
        mode.colors_max,
        mode.speed,
        mode.direction,
        mode.color_mode,
    ] {
        body.extend_from_slice(&v.to_le_bytes());
    }
    body.extend_from_slice(&(mode.colors.len() as u16).to_le_bytes());
    for c in &mode.colors {
        body.extend_from_slice(&[c.r, c.g, c.b, 0]);
    }
    let mut data = Vec::with_capacity(4 + body.len());
    data.extend_from_slice(&((4 + body.len()) as u32).to_le_bytes());
    data.extend_from_slice(&body);
    data
}

/// Curseur de lecture little-endian sur un buffer.
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.pos + n > self.buf.len() {
            bail!(
                "buffer OpenRGB tronqué (besoin {} octets à l'offset {}, taille {})",
                n,
                self.pos,
                self.buf.len()
            );
        }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }

    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    /// Chaîne OpenRGB : u16 longueur (null inclus), octets, null final.
    fn string(&mut self) -> Result<String> {
        let len = self.u16()? as usize;
        let bytes = self.take(len)?;
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
    }

    fn skip(&mut self, n: usize) -> Result<()> {
        self.take(n).map(|_| ())
    }
}

/// Parse la réponse REQUEST_CONTROLLER_DATA (protocole v1).
pub fn parse_controller_data(buf: &[u8]) -> Result<ControllerData> {
    let mut r = Reader::new(buf);
    r.u32().context("taille totale")?; // data_size dupliqué, ignoré
    let dtype = r.i32().context("type d'appareil")?;
    let name = r.string().context("nom")?;
    let vendor = r.string().context("vendor")?; // présent en v1+
    r.string().context("description")?;
    r.string().context("version")?;
    r.string().context("serial")?;
    r.string().context("location")?;

    let num_modes = r.u16().context("nb modes")?;
    let active_mode = r.i32().context("mode actif")?;
    let mut modes = Vec::with_capacity(num_modes as usize);
    for m in 0..num_modes {
        let mname = r.string().context("nom mode")?;
        let value = r.i32()?;
        let flags = r.u32()?;
        let speed_min = r.u32()?;
        let speed_max = r.u32()?;
        let colors_min = r.u32()?;
        let colors_max = r.u32()?;
        let speed = r.u32()?;
        let direction = r.u32()?;
        let color_mode = r.u32()?;
        let mode_colors = r.u16()?;
        let mut colors = Vec::with_capacity(mode_colors as usize);
        for _ in 0..mode_colors {
            let raw = r.take(4)?;
            colors.push(crate::core::Color::new(raw[0], raw[1], raw[2]));
        }
        modes.push(ModeInfo {
            index: m as u32,
            name: mname,
            value,
            flags,
            speed_min,
            speed_max,
            colors_min,
            colors_max,
            speed,
            direction,
            color_mode,
            colors,
        });
    }

    let num_zones = r.u16().context("nb zones")?;
    let mut zones = Vec::with_capacity(num_zones as usize);
    for _ in 0..num_zones {
        let zname = r.string().context("nom zone")?;
        r.i32()?; // zone type
        r.u32()?; // leds_min
        r.u32()?; // leds_max
        let leds_count = r.u32()?;
        let matrix_len = r.u16()? as usize;
        r.skip(matrix_len)?;
        zones.push(ZoneInfo {
            name: zname,
            led_count: leds_count,
        });
    }

    let num_leds = r.u16().context("nb leds")?;
    for _ in 0..num_leds {
        r.string()?; // nom LED
        r.u32()?; // valeur
    }

    let num_colors = r.u16().context("nb couleurs")?;
    r.skip(num_colors as usize * 4)?;

    Ok(ControllerData {
        device_type: DeviceType::from_openrgb(dtype),
        name,
        vendor,
        zones,
        led_count: num_leds as u32,
        modes,
        active_mode,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Color;

    fn push_str(buf: &mut Vec<u8>, s: &str) {
        let len = (s.len() + 1) as u16;
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend_from_slice(s.as_bytes());
        buf.push(0);
    }

    #[test]
    fn encode_update_mode_layout() {
        let mode = ModeInfo {
            index: 3,
            name: "Rainbow".into(),
            value: 7,
            flags: 0x41,
            speed_min: 0,
            speed_max: 100,
            colors_min: 1,
            colors_max: 2,
            speed: 50,
            direction: 1,
            color_mode: 1,
            colors: vec![Color::new(255, 0, 0), Color::new(0, 0, 255)],
        };
        let data = encode_update_mode(&mode);
        // data_size = taille totale du payload
        assert_eq!(
            u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize,
            data.len()
        );
        // mode_id
        assert_eq!(i32::from_le_bytes(data[4..8].try_into().unwrap()), 3);
        // nom : u16 longueur (null inclus) + octets + null
        assert_eq!(u16::from_le_bytes(data[8..10].try_into().unwrap()), 8);
        assert_eq!(&data[10..17], b"Rainbow");
        assert_eq!(data[17], 0);
        // value puis les 8 u32, puis nb couleurs et 2×4 octets couleur
        assert_eq!(i32::from_le_bytes(data[18..22].try_into().unwrap()), 7);
        let colors_off = 22 + 8 * 4;
        assert_eq!(
            u16::from_le_bytes(data[colors_off..colors_off + 2].try_into().unwrap()),
            2
        );
        assert_eq!(data.len(), colors_off + 2 + 2 * 4);
        assert_eq!(&data[colors_off + 2..colors_off + 6], &[255, 0, 0, 0]);
    }

    /// Construit un buffer controller_data v1 synthétique.
    fn build_controller(name: &str, vendor: &str, zones: &[(&str, u32)], leds: u32) -> Vec<u8> {
        let mut b: Vec<u8> = Vec::new();
        b.extend_from_slice(&0u32.to_le_bytes()); // data_size (ignoré)
        b.extend_from_slice(&5i32.to_le_bytes()); // type = Keyboard
        push_str(&mut b, name);
        push_str(&mut b, vendor);
        push_str(&mut b, "desc");
        push_str(&mut b, "1.0");
        push_str(&mut b, "SN123");
        push_str(&mut b, "HID: path");
        // 1 mode "Direct" avec 2 couleurs de mode
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&0i32.to_le_bytes()); // mode actif
        push_str(&mut b, "Direct");
        b.extend_from_slice(&0i32.to_le_bytes()); // value
        for _ in 0..8 {
            b.extend_from_slice(&0u32.to_le_bytes()); // flags..color_mode
        }
        b.extend_from_slice(&2u16.to_le_bytes()); // nb couleurs mode
        b.extend_from_slice(&[0u8; 8]);
        // zones
        b.extend_from_slice(&(zones.len() as u16).to_le_bytes());
        for (zn, zc) in zones {
            push_str(&mut b, zn);
            b.extend_from_slice(&0i32.to_le_bytes());
            b.extend_from_slice(&0u32.to_le_bytes());
            b.extend_from_slice(&zc.to_le_bytes());
            b.extend_from_slice(&zc.to_le_bytes());
            b.extend_from_slice(&0u16.to_le_bytes()); // pas de matrice
        }
        // leds
        b.extend_from_slice(&(leds as u16).to_le_bytes());
        for i in 0..leds {
            push_str(&mut b, &format!("LED {i}"));
            b.extend_from_slice(&0u32.to_le_bytes());
        }
        // couleurs
        b.extend_from_slice(&(leds as u16).to_le_bytes());
        b.extend_from_slice(&vec![0u8; leds as usize * 4]);
        b
    }

    #[test]
    fn parse_synthetic_controller() {
        let buf = build_controller("Clavier X", "VendorCo", &[("Main", 104)], 104);
        let c = parse_controller_data(&buf).unwrap();
        assert_eq!(c.name, "Clavier X");
        assert_eq!(c.vendor, "VendorCo");
        assert_eq!(c.device_type, DeviceType::Keyboard);
        assert_eq!(c.zones.len(), 1);
        assert_eq!(c.zones[0].led_count, 104);
        assert_eq!(c.led_count, 104);
    }

    #[test]
    fn parse_multi_zone() {
        let buf = build_controller("Hub", "V", &[("Ch1", 8), ("Ch2", 16)], 24);
        let c = parse_controller_data(&buf).unwrap();
        assert_eq!(c.zones.len(), 2);
        assert_eq!(c.zones[1].led_count, 16);
    }

    #[test]
    fn truncated_buffer_errors_cleanly() {
        let buf = build_controller("X", "V", &[("Z", 4)], 4);
        assert!(parse_controller_data(&buf[..20]).is_err());
    }

    #[test]
    fn header_layout() {
        let h = header(3, RGBCONTROLLER_UPDATELEDS, 10);
        assert_eq!(&h[0..4], b"ORGB");
        assert_eq!(u32::from_le_bytes(h[4..8].try_into().unwrap()), 3);
        assert_eq!(u32::from_le_bytes(h[8..12].try_into().unwrap()), 1050);
        assert_eq!(u32::from_le_bytes(h[12..16].try_into().unwrap()), 10);
    }

    #[test]
    fn update_leds_encoding() {
        let data = encode_update_leds(&[Color::new(255, 128, 64)]);
        assert_eq!(data.len(), 10);
        assert_eq!(u32::from_le_bytes(data[0..4].try_into().unwrap()), 10);
        assert_eq!(u16::from_le_bytes(data[4..6].try_into().unwrap()), 1);
        assert_eq!(&data[6..10], &[255, 128, 64, 0]);
    }
}
