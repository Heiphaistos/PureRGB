//! Backend OpenRGB : client TCP du serveur SDK OpenRGB (port 6742 par défaut).
//! Donne accès à tous les contrôleurs gérés par OpenRGB (900+ appareils).

pub mod manager;
pub mod protocol;
pub mod updater;

use crate::backends::Backend;
use crate::core::{Color, DeviceInfo, ModeInfo};
use anyhow::{bail, Context, Result};
use protocol as p;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub struct OpenRgbBackend {
    host: String,
    port: u16,
    stream: Option<TcpStream>,
    /// Nb de LEDs par contrôleur, indexé par id local, rempli au scan.
    led_counts: Vec<u32>,
    /// Contrôleurs déjà passés en mode custom (Direct).
    custom_mode_set: Vec<bool>,
    /// Modes matériels par contrôleur, remplis au scan (base pour UpdateMode).
    modes_cache: Vec<Vec<ModeInfo>>,
}

impl OpenRgbBackend {
    pub fn new(host: String, port: u16) -> Self {
        OpenRgbBackend {
            host,
            port,
            stream: None,
            led_counts: Vec::new(),
            custom_mode_set: Vec::new(),
            modes_cache: Vec::new(),
        }
    }

    /// Applique un mode matériel natif. `speed`/`direction`/`colors` sont des
    /// surcharges optionnelles du mode tel qu'annoncé par le contrôleur.
    pub fn set_mode(
        &mut self,
        local_id: &str,
        mode_index: u32,
        speed: Option<u32>,
        direction: Option<u32>,
        colors: Option<Vec<Color>>,
    ) -> Result<()> {
        let idx: u32 = local_id.parse().context("id OpenRGB invalide")?;
        self.connect()?;
        let mut mode = self
            .modes_cache
            .get(idx as usize)
            .and_then(|m| m.get(mode_index as usize))
            .with_context(|| format!("mode {mode_index} inconnu pour contrôleur {idx}"))?
            .clone();
        if let Some(s) = speed {
            mode.speed = s.clamp(mode.speed_min.min(mode.speed_max), mode.speed_min.max(mode.speed_max));
        }
        if let Some(d) = direction {
            mode.direction = d;
        }
        if let Some(c) = colors {
            let max = mode.colors_max.max(mode.colors_min).max(1) as usize;
            let min = mode.colors_min as usize;
            let mut c = c;
            c.truncate(max);
            if c.len() < min {
                c.resize(min, Color::BLACK);
            }
            if !c.is_empty() {
                // Passer en couleurs choisies par l'utilisateur si le mode le permet.
                if mode.flags & crate::core::MODE_FLAG_HAS_MODE_SPECIFIC_COLOR != 0 {
                    mode.color_mode = 1; // MODE_COLORS_MODE_SPECIFIC
                }
                mode.colors = c;
            }
        }
        let payload = p::encode_update_mode(&mode);
        self.send(idx, p::RGBCONTROLLER_UPDATEMODE, &payload)?;
        // Le firmware pilote désormais l'appareil : le mode Direct devra être
        // renégocié avant toute écriture LED directe.
        if let Some(flag) = self.custom_mode_set.get_mut(idx as usize) {
            *flag = false;
        }
        Ok(())
    }

    /// Redimensionne une zone ARGB (connecteur carte mère, canal de hub) au
    /// nombre de LEDs réellement branchées. Sans ça, OpenRGB laisse la zone
    /// à 0 LED et les ventilateurs/bandeaux branchés dessus restent invisibles.
    pub fn resize_zone(&mut self, local_id: &str, zone_idx: u32, new_size: u32) -> Result<()> {
        let idx: u32 = local_id.parse().context("id OpenRGB invalide")?;
        self.connect()?;
        let payload = p::encode_resize_zone(zone_idx, new_size);
        self.send(idx, p::RGBCONTROLLER_RESIZEZONE, &payload)?;
        // Le nombre total de LEDs du contrôleur a changé : invalider le cache
        // pour que set_colors retaille correctement au prochain envoi.
        if let Ok(c) = self.controller_data(idx) {
            if let Some(n) = self.led_counts.get_mut(idx as usize) {
                *n = c.led_count;
            }
        }
        Ok(())
    }

    pub fn set_endpoint(&mut self, host: String, port: u16) {
        self.host = host;
        self.port = port;
        self.stream = None;
    }

    fn connect(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Ok(());
        }
        let addr = format!("{}:{}", self.host, self.port);
        let stream = TcpStream::connect_timeout(
            &addr
                .parse()
                .or_else(|_| {
                    use std::net::ToSocketAddrs;
                    addr.to_socket_addrs()
                        .context("résolution DNS")?
                        .next()
                        .context("aucune adresse résolue")
                })
                .context("adresse serveur OpenRGB invalide")?,
            Duration::from_millis(1500),
        )
        .with_context(|| format!("connexion au serveur OpenRGB {addr}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        stream.set_nodelay(true)?;
        self.stream = Some(stream);

        // Handshake : version protocole puis nom client.
        self.send(0, p::REQUEST_PROTOCOL_VERSION, &p::PROTOCOL_VERSION.to_le_bytes())?;
        let (_, _, data) = self.recv_expect(p::REQUEST_PROTOCOL_VERSION)?;
        let server_ver = data
            .get(0..4)
            .and_then(|s| s.try_into().ok())
            .map(u32::from_le_bytes)
            .unwrap_or(0);
        log::info!("OpenRGB connecté, protocole serveur v{server_ver}");
        let mut name = b"PureRGB".to_vec();
        name.push(0);
        self.send(0, p::SET_CLIENT_NAME, &name)?;
        // Nouvelle connexion = serveur potentiellement relancé : les modes
        // custom déjà négociés ne sont plus valables.
        for flag in &mut self.custom_mode_set {
            *flag = false;
        }
        Ok(())
    }

    fn send(&mut self, device_id: u32, packet_id: u32, data: &[u8]) -> Result<()> {
        let stream = self.stream.as_mut().context("non connecté")?;
        let h = p::header(device_id, packet_id, data.len() as u32);
        let result = stream.write_all(&h).and_then(|_| stream.write_all(data));
        if result.is_err() {
            self.stream = None; // connexion morte, forcer une reconnexion
        }
        result.context("écriture socket OpenRGB")
    }

    /// Lit un paquet complet. Retourne (device_id, packet_id, data).
    fn recv(&mut self) -> Result<(u32, u32, Vec<u8>)> {
        let stream = self.stream.as_mut().context("non connecté")?;
        let mut h = [0u8; 16];
        if let Err(e) = stream.read_exact(&mut h) {
            self.stream = None;
            return Err(e).context("lecture en-tête OpenRGB");
        }
        if &h[0..4] != p::MAGIC {
            self.stream = None;
            bail!("magic OpenRGB invalide");
        }
        let device_id = u32::from_le_bytes(h[4..8].try_into().unwrap());
        let packet_id = u32::from_le_bytes(h[8..12].try_into().unwrap());
        let size = u32::from_le_bytes(h[12..16].try_into().unwrap()) as usize;
        if size > 16 * 1024 * 1024 {
            self.stream = None;
            bail!("paquet OpenRGB anormalement grand ({size} octets)");
        }
        let mut data = vec![0u8; size];
        if let Err(e) = self.stream.as_mut().unwrap().read_exact(&mut data) {
            self.stream = None;
            return Err(e).context("lecture données OpenRGB");
        }
        Ok((device_id, packet_id, data))
    }

    fn recv_expect(&mut self, packet_id: u32) -> Result<(u32, u32, Vec<u8>)> {
        // Le serveur peut intercaler des notifications (DeviceListUpdated = 100).
        for _ in 0..8 {
            let pkt = self.recv()?;
            if pkt.1 == packet_id {
                return Ok(pkt);
            }
        }
        bail!("réponse OpenRGB {packet_id} non reçue");
    }

    fn controller_count(&mut self) -> Result<u32> {
        self.send(0, p::REQUEST_CONTROLLER_COUNT, &[])?;
        let (_, _, data) = self.recv_expect(p::REQUEST_CONTROLLER_COUNT)?;
        if data.len() < 4 {
            bail!("réponse count trop courte");
        }
        Ok(u32::from_le_bytes(data[0..4].try_into().unwrap()))
    }

    fn controller_data(&mut self, idx: u32) -> Result<p::ControllerData> {
        self.send(idx, p::REQUEST_CONTROLLER_DATA, &p::PROTOCOL_VERSION.to_le_bytes())?;
        let (_, _, data) = self.recv_expect(p::REQUEST_CONTROLLER_DATA)?;
        p::parse_controller_data(&data)
    }
}

impl Backend for OpenRgbBackend {
    fn name(&self) -> &'static str {
        "openrgb"
    }

    fn scan(&mut self) -> Result<Vec<DeviceInfo>> {
        self.connect()?;
        let count = self.controller_count()?;
        let mut devices = Vec::with_capacity(count as usize);
        self.led_counts.clear();
        self.modes_cache.clear();
        self.custom_mode_set = vec![false; count as usize];
        for i in 0..count {
            match self.controller_data(i) {
                Ok(c) => {
                    self.led_counts.push(c.led_count);
                    self.modes_cache.push(c.modes.clone());
                    devices.push(DeviceInfo {
                        id: i.to_string(),
                        name: c.name,
                        vendor: c.vendor,
                        backend: String::new(), // rempli par le registre
                        device_type: c.device_type,
                        zones: c.zones,
                        led_count: c.led_count,
                        fan_channels: Vec::new(),
                        controllable: true,
                        has_lcd: false,
                        modes: c.modes,
                        active_mode: c.active_mode,
                        note: "via OpenRGB".into(),
                    });
                }
                Err(e) => {
                    log::warn!("contrôleur OpenRGB {i} illisible: {e:#}");
                    self.led_counts.push(0);
                    self.modes_cache.push(Vec::new());
                }
            }
        }
        Ok(devices)
    }

    fn set_colors(&mut self, local_id: &str, colors: &[Color]) -> Result<()> {
        let idx: u32 = local_id.parse().context("id OpenRGB invalide")?;
        self.connect()?;
        if let Some(set) = self.custom_mode_set.get_mut(idx as usize) {
            if !*set {
                // Mode Direct requis avant l'écriture LED directe.
                *set = true;
                let _ = self.send(idx, p::RGBCONTROLLER_SETCUSTOMMODE, &[]);
            }
        }
        // Tronquer/étendre au nombre de LEDs connu du contrôleur.
        let expected = self
            .led_counts
            .get(idx as usize)
            .copied()
            .unwrap_or(colors.len() as u32) as usize;
        let payload = if colors.len() == expected {
            p::encode_update_leds(colors)
        } else {
            let mut fixed = colors.to_vec();
            fixed.resize(expected, colors.last().copied().unwrap_or(Color::BLACK));
            p::encode_update_leds(&fixed)
        };
        self.send(idx, p::RGBCONTROLLER_UPDATELEDS, &payload)
    }

    fn is_available(&self) -> bool {
        self.stream.is_some()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
