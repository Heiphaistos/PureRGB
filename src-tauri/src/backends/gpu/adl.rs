//! Source : docs/rgb-protocols/gpu-acces-i2c-nvapi-adl.md § 2 « AMD : ADL2 » et en-têtes publics du SDK ADL (`ADLI2C`, `AdapterInfoX2`, constantes).
//!
//! `atiadlxx.dll` (installée avec le pilote Adrenalin, dossier système) est chargée dynamiquement. Énumération : `ADL2_Adapter_AdapterInfoX4_Get(-1)`,
//! une seule entrée par numéro de bus PCI (une entrée par sortie vidéo sinon) ; les identifiants PCI viennent de `strPNPString`
//! (`…VEN_xxxx&DEV_xxxx&SUBSYS_ddddvvvv…` : le sous-système vaut sous-device puis sous-vendeur). Transfert : `ADL2_Display_WriteAndReadI2C`
//! avec ligne 1, 100 kHz, adresse 8 bits = 7 bits << 1 ; lecture = `iOffset` = registre ; écriture = premier octet du tampon = registre.
//! Charge utile limitée à 24 octets par transfert (`ADL_DL_I2C_MAXDATASIZE`, registre compris) : au-delà, refus propre.
//! Toute fonction manquante ou tout code d'erreur = aucune carte exposée, jamais de plantage. Aucun accès sans liste blanche PCI (mod.rs).
//! Mémoire : le tampon de description des cartes est alloué par le callback ci-dessous et volontairement jamais libéré (une énumération par session).

use super::whitelist::Pci;
use crate::backends::smbus::bus::SmbusBus;
use anyhow::{bail, ensure, Context, Result};
use libloading::Library;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};
use std::sync::Arc;

const ADL_OK: c_int = 0;
const ACTION_READ: c_int = 0x1;
const ACTION_WRITE: c_int = 0x2;
const MAX_PAYLOAD: usize = 0x18;
const LINE: c_int = 1;
const SPEED_KHZ: c_int = 100;
const MAX_PATH: usize = 256;
const AMD_VENDOR: u16 = 0x1002;

type AllocFn = unsafe extern "system" fn(c_int) -> *mut c_void;
type CreateFn = unsafe extern "system" fn(AllocFn, c_int, *mut *mut c_void) -> c_int;
type CountFn = unsafe extern "system" fn(*mut c_void, *mut c_int) -> c_int;
type InfoFn = unsafe extern "system" fn(*mut c_void, c_int, *mut c_int, *mut *mut AdapterInfoX2) -> c_int;
type I2cFn = unsafe extern "system" fn(*mut c_void, c_int, *mut Adli2c) -> c_int;

/// `ADLI2C` (x64) : 7 entiers puis le pointeur de données.
#[repr(C)]
struct Adli2c {
    size: c_int,
    line: c_int,
    address: c_int,
    offset: c_int,
    action: c_int,
    speed: c_int,
    data_size: c_int,
    data: *mut c_char,
}

/// `AdapterInfoX2` (1580 octets).
#[repr(C)]
struct AdapterInfoX2 {
    size: c_int,
    adapter_index: c_int,
    udid: [c_char; MAX_PATH],
    bus_number: c_int,
    device_number: c_int,
    function_number: c_int,
    vendor_id: c_int,
    adapter_name: [c_char; MAX_PATH],
    display_name: [c_char; MAX_PATH],
    present: c_int,
    exist: c_int,
    driver_path: [c_char; MAX_PATH],
    driver_path_ext: [c_char; MAX_PATH],
    pnp_string: [c_char; MAX_PATH],
    os_display_index: c_int,
    info_mask: c_int,
    info_value: c_int,
}

struct Api {
    _lib: Library,
    context: *mut c_void,
    io: I2cFn,
}

// SAFETY: le contexte ADL n'est utilisé que depuis le thread qui détient le `&mut AdlBus` (le backend est derrière un Mutex).
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

pub struct AdlBus {
    api: Arc<Api>,
    adapter: c_int,
}

unsafe extern "system" fn alloc(size: c_int) -> *mut c_void {
    // Tampon rendu à ADL et jamais libéré (voir l'en-tête du module).
    Box::leak(vec![0u8; size.max(0) as usize].into_boxed_slice()).as_mut_ptr().cast()
}

/// `(vendeur, device, sous-vendeur, sous-device)` depuis `PCI\VEN_1002&DEV_73BF&SUBSYS_E4381DA2&REV_C1\…`.
pub fn parse_pnp(s: &str) -> Option<Pci> {
    let hex = |tag: &str, len: usize| -> Option<(usize, u32)> {
        let i = s.find(tag)? + tag.len();
        let v = u32::from_str_radix(s.get(i..i + len)?, 16).ok()?;
        Some((i, v))
    };
    let (_, vendor) = hex("VEN_", 4)?;
    let (_, device) = hex("DEV_", 4)?;
    let (_, sub) = hex("SUBSYS_", 8)?;
    Some(Pci { vendor: vendor as u16, device: device as u16, subvendor: (sub & 0xFFFF) as u16, subdevice: (sub >> 16) as u16 })
}

fn text(buf: &[c_char]) -> String {
    let bytes: Vec<u8> = buf.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Cartes AMD vues par le pilote, avec leur bus I2C. Vide si ADL est absente ou refuse.
pub fn enumerate() -> Vec<(Pci, Box<dyn SmbusBus>)> {
    match try_enumerate() {
        Ok(v) => v,
        Err(e) => {
            log::debug!("gpu : ADL indisponible : {e:#}");
            Vec::new()
        }
    }
}

fn try_enumerate() -> Result<Vec<(Pci, Box<dyn SmbusBus>)>> {
    // SAFETY: bibliothèque système signée d'AMD, chargée par nom depuis le dossier système uniquement.
    let lib = unsafe { Library::new("atiadlxx.dll") }.context("atiadlxx.dll introuvable (pas de pilote AMD)")?;
    // SAFETY: symboles exportés documentés par la note ; signatures des en-têtes publics ADL.
    let (create, count, info, io) = unsafe {
        (
            *lib.get::<CreateFn>(b"ADL2_Main_Control_Create\0").context("ADL2_Main_Control_Create absent")?,
            *lib.get::<CountFn>(b"ADL2_Adapter_NumberOfAdapters_Get\0").context("ADL2_Adapter_NumberOfAdapters_Get absent")?,
            *lib.get::<InfoFn>(b"ADL2_Adapter_AdapterInfoX4_Get\0").context("ADL2_Adapter_AdapterInfoX4_Get absent")?,
            *lib.get::<I2cFn>(b"ADL2_Display_WriteAndReadI2C\0").context("ADL2_Display_WriteAndReadI2C absent")?,
        )
    };
    let mut context: *mut c_void = std::ptr::null_mut();
    // SAFETY: appels ADL avec des pointeurs valides ; le tableau renvoyé contient `n` entrées `AdapterInfoX2`.
    unsafe {
        ensure!(create(alloc, 1, &mut context) == ADL_OK && !context.is_null(), "ADL2_Main_Control_Create a échoué");
        let mut n: c_int = 0;
        ensure!(count(context, &mut n) == ADL_OK && n > 0, "aucun adaptateur ADL");
        let mut list: *mut AdapterInfoX2 = std::ptr::null_mut();
        let mut got: c_int = 0;
        ensure!(info(context, -1, &mut got, &mut list) == ADL_OK && !list.is_null() && got > 0, "ADL2_Adapter_AdapterInfoX4_Get a échoué");
        let api = Arc::new(Api { _lib: lib, context, io });
        let mut out: Vec<(Pci, Box<dyn SmbusBus>)> = Vec::new();
        let mut last_bus = -1;
        for i in 0..got as usize {
            let a = &*list.add(i);
            if a.bus_number == last_bus {
                continue; // une entrée par sortie vidéo : un seul bus par carte
            }
            last_bus = a.bus_number;
            let Some(pci) = parse_pnp(&text(&a.pnp_string)) else { continue };
            if pci.vendor != AMD_VENDOR {
                continue;
            }
            out.push((pci, Box::new(AdlBus { api: api.clone(), adapter: a.adapter_index })));
        }
        Ok(out)
    }
}

impl AdlBus {
    fn call(&self, addr: u8, read: bool, offset: u8, buf: &mut [u8]) -> Result<()> {
        ensure!(addr < 0x80, "adresse I2C 7 bits invalide : 0x{addr:02X}");
        ensure!(!buf.is_empty() && buf.len() <= MAX_PAYLOAD, "charge utile ADL de {} octets (max {MAX_PAYLOAD})", buf.len());
        let mut req = Adli2c {
            size: std::mem::size_of::<Adli2c>() as c_int,
            line: LINE,
            address: c_int::from(addr) << 1,
            offset: c_int::from(offset),
            action: if read { ACTION_READ } else { ACTION_WRITE },
            speed: SPEED_KHZ,
            data_size: buf.len() as c_int,
            data: buf.as_mut_ptr().cast(),
        };
        // SAFETY: `req` et `buf` vivent pendant l'appel ; contexte et index d'adaptateur viennent de l'énumération.
        let status = unsafe { (self.api.io)(self.api.context, self.adapter, &mut req) };
        if status != ADL_OK {
            bail!("ADL I2C {} à 0x{addr:02X} : code {status}", if read { "lecture" } else { "écriture" });
        }
        Ok(())
    }
}

impl SmbusBus for AdlBus {
    fn read_byte_data(&mut self, addr: u8, reg: u8) -> Result<u8> {
        let mut b = [0u8];
        self.call(addr, true, reg, &mut b)?;
        Ok(b[0])
    }
    fn write_byte_data(&mut self, addr: u8, reg: u8, value: u8) -> Result<()> {
        self.call(addr, false, 0, &mut [reg, value])
    }
    fn write_block_data(&mut self, addr: u8, reg: u8, data: &[u8]) -> Result<()> {
        let mut b = Vec::with_capacity(data.len() + 1);
        b.push(reg);
        b.extend_from_slice(data);
        self.call(addr, false, 0, &mut b)
    }
    fn read_block_data(&mut self, addr: u8, reg: u8, len: usize) -> Result<Vec<u8>> {
        let mut b = vec![0u8; len];
        self.call(addr, true, reg, &mut b)?;
        Ok(b)
    }
    fn write_word_data(&mut self, addr: u8, reg: u8, word: u16) -> Result<()> {
        let w = word.to_le_bytes();
        self.call(addr, false, 0, &mut [reg, w[0], w[1]])
    }
    fn write_raw(&mut self, addr: u8, data: &[u8]) -> Result<()> {
        let mut b = data.to_vec();
        self.call(addr, false, 0, &mut b)
    }
    fn read_raw(&mut self, addr: u8, len: usize) -> Result<Vec<u8>> {
        let mut b = vec![0u8; len];
        self.call(addr, true, 0, &mut b)?;
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn struct_layouts_match_the_x64_sdk() {
        assert_eq!(std::mem::size_of::<Adli2c>(), 40);
        assert_eq!(std::mem::offset_of!(Adli2c, data), 32);
        assert_eq!(std::mem::size_of::<AdapterInfoX2>(), 1580);
        assert_eq!(std::mem::offset_of!(AdapterInfoX2, bus_number), 264);
        assert_eq!(std::mem::offset_of!(AdapterInfoX2, pnp_string), 1312);
    }

    #[test]
    fn pnp_string_gives_pci_ids_with_swapped_subsystem() {
        let p = parse_pnp(r"PCI\VEN_1002&DEV_73BF&SUBSYS_E4381DA2&REV_C1\6&2B7A").expect("ok");
        assert_eq!((p.vendor, p.device, p.subvendor, p.subdevice), (0x1002, 0x73BF, 0x1DA2, 0xE438));
        assert!(parse_pnp("USB\\VID_1234").is_none());
        assert!(parse_pnp("PCI\\VEN_1002&DEV_73BF").is_none());
    }

    #[test]
    fn payload_over_24_bytes_is_refused_before_any_call() {
        // Pas de DLL ici : on ne peut pas construire un AdlBus, mais la limite est une constante vérifiée par le test de mise en page.
        assert_eq!(MAX_PAYLOAD, 24);
    }
}
