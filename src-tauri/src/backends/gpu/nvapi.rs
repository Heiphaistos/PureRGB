//! Source : docs/rgb-protocols/gpu-acces-i2c-nvapi-adl.md § 1 « NVIDIA : NVAPI I2C Ex » (identifiants de fonctions,
//! structure `NV_I2C_INFO_V3`, port 1, adresse 8 bits = 7 bits << 1, traduction SMBus -> NVAPI).
//!
//! `nvapi64.dll` (installée avec le pilote GeForce, dossier système) est chargée dynamiquement ; chaque fonction s'obtient
//! par `nvapi_QueryInterface(id)`. Interfaces non publiques : tout identifiant introuvable ou tout code d'erreur = refus
//! propre (aucune carte exposée), jamais de plantage. Aucun accès sans correspondance de la liste blanche PCI (mod.rs).
//! INCONNU : 3e argument de `NvAPI_I2C*Ex` (entier non documenté, passé à 0 comme la source de référence) ;
//! disposition de `GetPCIIdentifiers` (device_id : poids faible = vendeur ; sub_system_id : poids faible = sous-vendeur).

use super::whitelist::Pci;
use crate::backends::smbus::bus::SmbusBus;
use anyhow::{bail, ensure, Context, Result};
use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::sync::Arc;

const ID_INITIALIZE: u32 = 0x0150_E828;
const ID_ENUM_GPUS: u32 = 0xE5AC_921F;
const ID_PCI_IDS: u32 = 0x2DDF_B66E;
const ID_I2C_WRITE_EX: u32 = 0x283A_C65A;
const ID_I2C_READ_EX: u32 = 0x4D7B_0709;
const MAX_GPUS: usize = 64;
const PORT_ID: u8 = 1;

type QueryFn = unsafe extern "C" fn(u32) -> *const c_void;
type InitFn = unsafe extern "C" fn() -> i32;
type EnumFn = unsafe extern "C" fn(*mut *mut c_void, *mut u32) -> i32;
type PciFn = unsafe extern "C" fn(*mut c_void, *mut u32, *mut u32, *mut u32, *mut u32) -> i32;
type I2cFn = unsafe extern "C" fn(*mut c_void, *mut I2cInfo, *mut u32) -> i32;

/// `NV_I2C_INFO_V3` (x64) : 64 octets, version = taille | 3 << 16.
#[repr(C)]
struct I2cInfo {
    version: u32,
    display_mask: u32,
    is_ddc_port: u8,
    dev_address: u8,
    reg_address: *mut u8,
    reg_addr_size: u32,
    data: *mut u8,
    size: u32,
    speed: u32,
    speed_khz: u32,
    port_id: u8,
    is_port_id_set: u32,
}

struct Api {
    _lib: Library,
    write: I2cFn,
    read: I2cFn,
}

pub struct NvBus {
    api: Arc<Api>,
    handle: *mut c_void,
}

// SAFETY: le handle de GPU n'est utilisé que par le thread qui détient le `&mut NvBus` (le backend est derrière un Mutex).
unsafe impl Send for NvBus {}

fn api_fn<T: Copy>(query: &Symbol<QueryFn>, id: u32) -> Result<T> {
    // SAFETY: `query` est `nvapi_QueryInterface` ; le pointeur renvoyé est une fonction du type voulu par l'identifiant.
    let p = unsafe { query(id) };
    ensure!(!p.is_null(), "fonction NVAPI 0x{id:08X} absente de ce pilote");
    // SAFETY: les signatures ci-dessus sont celles de la note ; `T` est un pointeur de fonction de même taille que `*const c_void`.
    Ok(unsafe { std::mem::transmute_copy::<*const c_void, T>(&p) })
}

/// Cartes NVIDIA vues par le pilote, avec leur bus I2C. Vide si NVAPI est absente ou refuse.
pub fn enumerate() -> Vec<(Pci, Box<dyn SmbusBus>)> {
    match try_enumerate() {
        Ok(v) => v,
        Err(e) => {
            log::debug!("gpu : NVAPI indisponible : {e:#}");
            Vec::new()
        }
    }
}

fn try_enumerate() -> Result<Vec<(Pci, Box<dyn SmbusBus>)>> {
    // SAFETY: bibliothèque système signée de NVIDIA, chargée par nom depuis le dossier système uniquement.
    let lib = unsafe { Library::new("nvapi64.dll") }.context("nvapi64.dll introuvable (pas de pilote NVIDIA)")?;
    let (init, enum_gpus, pci_ids, write, read) = {
        // SAFETY: symbole exporté documenté par la note.
        let query: Symbol<QueryFn> = unsafe { lib.get(b"nvapi_QueryInterface\0") }.context("nvapi_QueryInterface absent")?;
        (
            api_fn::<InitFn>(&query, ID_INITIALIZE)?,
            api_fn::<EnumFn>(&query, ID_ENUM_GPUS)?,
            api_fn::<PciFn>(&query, ID_PCI_IDS)?,
            api_fn::<I2cFn>(&query, ID_I2C_WRITE_EX)?,
            api_fn::<I2cFn>(&query, ID_I2C_READ_EX)?,
        )
    };
    // SAFETY: appels NVAPI avec des tampons de la taille attendue (64 handles).
    unsafe {
        ensure!(init() == 0, "NvAPI_Initialize a échoué");
        let mut handles = [std::ptr::null_mut::<c_void>(); MAX_GPUS];
        let mut count = 0u32;
        ensure!(enum_gpus(handles.as_mut_ptr(), &mut count) == 0, "NvAPI_EnumPhysicalGPUs a échoué");
        let api = Arc::new(Api { _lib: lib, write, read });
        let mut out: Vec<(Pci, Box<dyn SmbusBus>)> = Vec::new();
        for h in handles.iter().take((count as usize).min(MAX_GPUS)) {
            let (mut dev, mut sub, mut rev, mut ext) = (0u32, 0u32, 0u32, 0u32);
            if pci_ids(*h, &mut dev, &mut sub, &mut rev, &mut ext) != 0 {
                continue;
            }
            let pci = Pci { vendor: (dev & 0xFFFF) as u16, device: (dev >> 16) as u16, subvendor: (sub & 0xFFFF) as u16, subdevice: (sub >> 16) as u16 };
            out.push((pci, Box::new(NvBus { api: api.clone(), handle: *h })));
        }
        Ok(out)
    }
}

impl NvBus {
    fn call(&self, write: bool, addr: u8, reg: Option<u8>, data: &mut [u8]) -> Result<()> {
        ensure!(addr < 0x80, "adresse I2C 7 bits invalide : 0x{addr:02X}");
        let mut reg_byte = reg.unwrap_or(0);
        let mut info = I2cInfo {
            version: (std::mem::size_of::<I2cInfo>() as u32) | (3 << 16),
            display_mask: 0,
            is_ddc_port: 0,
            dev_address: addr << 1,
            reg_address: if reg.is_some() { &mut reg_byte } else { std::ptr::null_mut() },
            reg_addr_size: u32::from(reg.is_some()),
            data: data.as_mut_ptr(),
            size: data.len() as u32,
            speed: 0xFFFF,
            speed_khz: 0,
            port_id: PORT_ID,
            is_port_id_set: 1,
        };
        let mut unknown = 0u32;
        let f = if write { self.api.write } else { self.api.read };
        // SAFETY: `info` et les tampons vivent pendant l'appel ; `handle` vient de `NvAPI_EnumPhysicalGPUs`.
        let status = unsafe { f(self.handle, &mut info, &mut unknown) };
        if status != 0 {
            bail!("NVAPI I2C {} à 0x{addr:02X} : code {status}", if write { "écriture" } else { "lecture" });
        }
        Ok(())
    }
}

impl SmbusBus for NvBus {
    fn read_byte_data(&mut self, addr: u8, reg: u8) -> Result<u8> {
        let mut b = [0u8];
        self.call(false, addr, Some(reg), &mut b)?;
        Ok(b[0])
    }
    fn write_byte_data(&mut self, addr: u8, reg: u8, value: u8) -> Result<()> {
        self.call(true, addr, Some(reg), &mut [value])
    }
    fn write_block_data(&mut self, addr: u8, reg: u8, data: &[u8]) -> Result<()> {
        let mut d = data.to_vec();
        self.call(true, addr, Some(reg), &mut d)
    }
    fn write_word_data(&mut self, addr: u8, reg: u8, word: u16) -> Result<()> {
        self.call(true, addr, Some(reg), &mut word.to_le_bytes())
    }
    fn write_raw(&mut self, addr: u8, data: &[u8]) -> Result<()> {
        let mut d = data.to_vec();
        self.call(true, addr, None, &mut d)
    }
    fn read_raw(&mut self, addr: u8, len: usize) -> Result<Vec<u8>> {
        ensure!((1..=64).contains(&len), "longueur de lecture invalide : {len}");
        let mut d = vec![0u8; len];
        self.call(false, addr, None, &mut d)?;
        Ok(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i2c_info_layout_matches_the_x64_header() {
        assert_eq!(std::mem::size_of::<I2cInfo>(), 64);
        assert_eq!(std::mem::offset_of!(I2cInfo, reg_address), 16);
        assert_eq!(std::mem::offset_of!(I2cInfo, data), 32);
        assert_eq!(std::mem::offset_of!(I2cInfo, port_id), 52);
        assert_eq!(std::mem::offset_of!(I2cInfo, is_port_id_set), 56);
    }

    /// Essai réel, LECTURE SEULE (aucun transfert I2C) : `cargo test --lib nvapi_live -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn nvapi_live_lists_pci_ids() {
        let cards = enumerate();
        for (p, _) in &cards {
            println!("GPU PCI {:04x}:{:04x} sous-système {:04x}:{:04x}", p.vendor, p.device, p.subvendor, p.subdevice);
        }
        assert!(!cards.is_empty());
    }
}
