//! CRC-32 des rapports de sortie Bluetooth PlayStation (DualSense 0x31, DualShock 4 0x11).
//! Source : driver Linux `drivers/hid/hid-playstation.c` (branche master, consulté 2026-10-25), fonction d'envoi des
//! rapports : `crc = crc32_le(0xFFFFFFFF, &seed, 1); crc = ~crc32_le(crc, data, len - 4)` avec `seed = 0xA2`
//! (PS_OUTPUT_CRC32_SEED), résultat écrit en little-endian dans les 4 derniers octets. Le CRC couvre l'octet
//! d'identifiant de rapport. Réimplémenté ici d'après cette description (CRC-32 IEEE réfléchi, polynôme 0xEDB88320).

pub const OUTPUT_SEED: u8 = 0xA2;

fn crc32_update(mut crc: u32, data: &[u8]) -> u32 {
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    crc
}

/// Remplit les 4 derniers octets de `report` avec le CRC du reste, préfixé de la graine de sortie.
pub fn seal_output(report: &mut [u8]) {
    let n = report.len().saturating_sub(4);
    let crc = !crc32_update(crc32_update(0xFFFF_FFFF, &[OUTPUT_SEED]), &report[..n]);
    report[n..].copy_from_slice(&crc.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_standard_check_value() {
        // Valeur de contrôle publique de CRC-32/IEEE : "123456789" -> 0xCBF43926.
        assert_eq!(!crc32_update(0xFFFF_FFFF, b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn seal_prefixes_the_seed_and_writes_little_endian() {
        // zlib.crc32(b"\xa2123456789") = 0x63DA9F12, calculé indépendamment (Python).
        let mut r = *b"123456789\0\0\0\0";
        seal_output(&mut r);
        assert_eq!(&r[9..], &[0x12, 0x9F, 0xDA, 0x63]);
    }
}
