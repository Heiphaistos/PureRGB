//! Table de détection des contrôleurs RGB/ventilation USB connus.
//! Sert à identifier le matériel présent même sans driver natif
//! (l'utilisateur sait alors quoi brancher sur le pont OpenRGB).

use crate::core::DeviceType;

pub struct KnownDevice {
    pub vid: u16,
    pub pid: u16,
    pub name: &'static str,
    pub device_type: DeviceType,
    /// Driver natif expérimental disponible dans PureRGB.
    pub native_driver: Option<NativeDriver>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeDriver {
    CorsairLightingNode,
    NzxtHue2,
}

pub const KNOWN_DEVICES: &[KnownDevice] = &[
    // ---- Corsair (0x1B1C) ----
    KnownDevice { vid: 0x1B1C, pid: 0x0C0B, name: "Corsair Lighting Node Pro", device_type: DeviceType::Hub, native_driver: Some(NativeDriver::CorsairLightingNode) },
    KnownDevice { vid: 0x1B1C, pid: 0x0C1A, name: "Corsair Lighting Node Core", device_type: DeviceType::Hub, native_driver: Some(NativeDriver::CorsairLightingNode) },
    KnownDevice { vid: 0x1B1C, pid: 0x0C10, name: "Corsair Commander Pro", device_type: DeviceType::Hub, native_driver: None },
    KnownDevice { vid: 0x1B1C, pid: 0x0C1C, name: "Corsair Commander Core", device_type: DeviceType::Hub, native_driver: None },
    KnownDevice { vid: 0x1B1C, pid: 0x0C2A, name: "Corsair Commander Core XT", device_type: DeviceType::Hub, native_driver: None },
    KnownDevice { vid: 0x1B1C, pid: 0x0C3F, name: "Corsair iCUE Link System Hub", device_type: DeviceType::Hub, native_driver: None },
    KnownDevice { vid: 0x1B1C, pid: 0x0C12, name: "Corsair Hydro H150i Pro", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x1B1C, pid: 0x0C21, name: "Corsair Hydro Platinum / Pro XT", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x1B1C, pid: 0x0C35, name: "Corsair iCUE Elite Capellix", device_type: DeviceType::Aio, native_driver: None },
    // ---- NZXT (0x1E71) — PIDs alignés sur liquidctl ----
    KnownDevice { vid: 0x1E71, pid: 0x2006, name: "NZXT Smart Device V2", device_type: DeviceType::Hub, native_driver: Some(NativeDriver::NzxtHue2) },
    KnownDevice { vid: 0x1E71, pid: 0x1714, name: "NZXT Smart Device V1", device_type: DeviceType::Hub, native_driver: None },
    KnownDevice { vid: 0x1E71, pid: 0x1711, name: "NZXT Grid+ V3", device_type: DeviceType::Hub, native_driver: None },
    KnownDevice { vid: 0x1E71, pid: 0x170E, name: "NZXT Kraken X42/X52/X62/X72", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x1E71, pid: 0x1715, name: "NZXT Kraken M22", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x1E71, pid: 0x2007, name: "NZXT Kraken X53/X63/X73", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x1E71, pid: 0x3008, name: "NZXT Kraken Z53/Z63/Z73", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x1E71, pid: 0x300C, name: "NZXT Kraken 2023", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x1E71, pid: 0x300E, name: "NZXT Kraken 2023 Elite", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x1E71, pid: 0x2009, name: "NZXT RGB & Fan Controller", device_type: DeviceType::Hub, native_driver: Some(NativeDriver::NzxtHue2) },
    KnownDevice { vid: 0x1E71, pid: 0x200E, name: "NZXT RGB & Fan Controller", device_type: DeviceType::Hub, native_driver: Some(NativeDriver::NzxtHue2) },
    KnownDevice { vid: 0x1E71, pid: 0x2019, name: "NZXT RGB & Fan Controller", device_type: DeviceType::Hub, native_driver: Some(NativeDriver::NzxtHue2) },
    KnownDevice { vid: 0x1E71, pid: 0x2020, name: "NZXT RGB & Fan Controller V2", device_type: DeviceType::Hub, native_driver: Some(NativeDriver::NzxtHue2) },
    KnownDevice { vid: 0x1E71, pid: 0x2001, name: "NZXT HUE 2", device_type: DeviceType::LedStrip, native_driver: Some(NativeDriver::NzxtHue2) },
    KnownDevice { vid: 0x1E71, pid: 0x2002, name: "NZXT HUE 2 Ambient", device_type: DeviceType::LedStrip, native_driver: None },
    // ---- ASUS (0x0B05) ----
    KnownDevice { vid: 0x0B05, pid: 0x1867, name: "ASUS Aura LED Controller", device_type: DeviceType::Motherboard, native_driver: None },
    KnownDevice { vid: 0x0B05, pid: 0x1872, name: "ASUS Aura LED Controller", device_type: DeviceType::Motherboard, native_driver: None },
    KnownDevice { vid: 0x0B05, pid: 0x18A3, name: "ASUS Aura Addressable", device_type: DeviceType::Motherboard, native_driver: None },
    KnownDevice { vid: 0x0B05, pid: 0x18A5, name: "ASUS Aura Addressable", device_type: DeviceType::Motherboard, native_driver: None },
    KnownDevice { vid: 0x0B05, pid: 0x19AF, name: "ASUS ROG Ryujin AIO", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x0B05, pid: 0x1AA6, name: "ASUS ROG Strix AIO", device_type: DeviceType::Aio, native_driver: None },
    // ---- Gigabyte / ITE (0x048D) ----
    KnownDevice { vid: 0x048D, pid: 0x8297, name: "Gigabyte RGB Fusion 2 (carte mère)", device_type: DeviceType::Motherboard, native_driver: None },
    KnownDevice { vid: 0x048D, pid: 0x5702, name: "Gigabyte RGB Fusion (ITE 5702)", device_type: DeviceType::Motherboard, native_driver: None },
    KnownDevice { vid: 0x048D, pid: 0x5711, name: "Gigabyte RGB Fusion (ITE 5711)", device_type: DeviceType::Motherboard, native_driver: None },
    // ---- Lian Li (0x0CF2) ----
    KnownDevice { vid: 0x0CF2, pid: 0x7750, name: "Lian Li Uni Hub SL", device_type: DeviceType::Hub, native_driver: None },
    KnownDevice { vid: 0x0CF2, pid: 0xA100, name: "Lian Li Uni Hub AL", device_type: DeviceType::Hub, native_driver: None },
    KnownDevice { vid: 0x0CF2, pid: 0xA102, name: "Lian Li Uni Hub SL Infinity", device_type: DeviceType::Hub, native_driver: None },
    KnownDevice { vid: 0x0CF2, pid: 0xA103, name: "Lian Li Uni Hub SL v2", device_type: DeviceType::Hub, native_driver: None },
    // ---- Thermaltake (0x264A) ----
    KnownDevice { vid: 0x264A, pid: 0x1FA5, name: "Thermaltake Riing Plus", device_type: DeviceType::Hub, native_driver: None },
    KnownDevice { vid: 0x264A, pid: 0x2260, name: "Thermaltake Riing Quad", device_type: DeviceType::Hub, native_driver: None },
    // ---- Cooler Master (0x2516) ----
    KnownDevice { vid: 0x2516, pid: 0x004F, name: "Cooler Master ARGB Controller", device_type: DeviceType::Hub, native_driver: None },
    KnownDevice { vid: 0x2516, pid: 0x0173, name: "Cooler Master ARGB Gen2", device_type: DeviceType::Hub, native_driver: None },
    // ---- ASRock (0x26CE) ----
    KnownDevice { vid: 0x26CE, pid: 0x01A2, name: "ASRock Polychrome LED Controller", device_type: DeviceType::Motherboard, native_driver: None },
    // ---- Corsair AIO (compléments liquidctl) ----
    KnownDevice { vid: 0x1B1C, pid: 0x0C18, name: "Corsair H100i Platinum", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x1B1C, pid: 0x0C17, name: "Corsair H115i Platinum", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x1B1C, pid: 0x0C20, name: "Corsair H100i Pro XT", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x1B1C, pid: 0x0C22, name: "Corsair H150i Pro XT", device_type: DeviceType::Aio, native_driver: None },
    KnownDevice { vid: 0x1B1C, pid: 0x1D00, name: "Corsair Obsidian 1000D", device_type: DeviceType::Case, native_driver: None },
];

/// Fabricants reconnus au VID seul (détection générique).
pub const KNOWN_VENDORS: &[(u16, &str, DeviceType)] = &[
    (0x1B1C, "Corsair", DeviceType::Unknown),
    (0x1E71, "NZXT", DeviceType::Unknown),
    (0x0B05, "ASUS", DeviceType::Unknown),
    (0x1462, "MSI", DeviceType::Unknown),
    (0x048D, "Gigabyte (ITE)", DeviceType::Unknown),
    (0x1532, "Razer", DeviceType::Unknown),
    (0x046D, "Logitech", DeviceType::Unknown),
    (0x1038, "SteelSeries", DeviceType::Unknown),
    (0x0951, "HyperX", DeviceType::Unknown),
    (0x0CF2, "Lian Li (ENE)", DeviceType::Unknown),
    (0x264A, "Thermaltake", DeviceType::Unknown),
    (0x2516, "Cooler Master", DeviceType::Unknown),
    (0x3633, "DeepCool", DeviceType::Unknown),
    (0x3842, "EVGA", DeviceType::Unknown),
    (0x26CE, "ASRock", DeviceType::Unknown),
];

pub fn find_known(vid: u16, pid: u16) -> Option<&'static KnownDevice> {
    KNOWN_DEVICES.iter().find(|d| d.vid == vid && d.pid == pid)
}

pub fn find_vendor(vid: u16) -> Option<&'static (u16, &'static str, DeviceType)> {
    KNOWN_VENDORS.iter().find(|(v, _, _)| *v == vid)
}
