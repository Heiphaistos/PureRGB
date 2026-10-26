# Températures de la RAM par SMBus (DDR4 TSE2004, DDR5 SPD5118), lecture seule

Complète `smbus-i2c-acces-et-securite.md` (bus, PawnIO, mutex `Global\Access_SMBUS.HTP.Method`). Implémentation : `src-tauri/src/backends/smbus/ramtemp.rs`.

## Sources et limites
- JEDEC JESD300-5B (SPD5118 hub), JEDEC 21-C §4.1.6 / JC-42.4 (TSE2004 : capteur de température des modules DDR4), datasheets SPD5118 et TSE2004.
  Ces normes ne sont pas toutes publiques : les valeurs ci-dessous sont **recoupées avec les pilotes noyau Linux publics** `drivers/hwmon/spd5118.c`
  et `drivers/hwmon/jc42.c` (lus le 2026-10-26) ; seuls des faits de protocole sont repris, aucun code.
- **iCUE n'a PAS été analysé** (non téléchargé) : on ne sait pas comment il lit les températures. Rien ici ne s'en inspire. INCONNU (non vérifié).
- Pas d'essai sur matériel réel : tests uniquement avec un bus simulé (octets faits main).

## Bus et adresses (7 bits)
Même bus SMBus chipset que le RGB (Intel i801 / AMD PIIX4 via PawnIO). Emplacement n (0..7) :

| Génération | Composant | Adresse |
|---|---|---|
| DDR4 | EEPROM SPD (reconnaissance) | 0x50+n, octet 2 = 0x0C (type DRAM DDR4) |
| DDR4 | capteur TSE2004 | 0x18+n |
| DDR5 | hub SPD5118 (reconnaissance et température) | 0x50+n |

## DDR4 : TSE2004
- Registre de température 0x05 (jc42.c `JC42_REG_TEMP`), lecture « mot » SMBus : le capteur envoie l'octet de poids fort d'abord, donc le mot reçu est à échanger (jc42 : `read_word_swapped`).
- Bits 12:0 : valeur signée en complément à 2 sur 13 bits, **LSB = 0,0625 °C** (jc42 : `sign_extend32(reg,12) * 125 / 2` m°C) ; les composants à résolution 0,25 °C laissent les 2 bits bas à 0. Bits 15:13 = drapeaux de seuils, ignorés.

## DDR5 : SPD5118
- Identité : MR0:MR1 = `0x51 0x18` (spd5118.c ; OpenRGB teste `(MR1 & 0xEF) == 0x08`, accepté aussi).
- MR5 (0x05) bit 1 = capteur présent ; MR26 (0x1A) bit 0 = capteur désactivé (`SPD5118_TS_DISABLE`) -> barrette ignorée, **on ne le réactive jamais**.
- Température : MR49 (0x31) = octet bas, MR50 (0x32) = octet haut (spd5118.c `SPD5118_REG_TEMP`, poids faible d'abord). Bits 12:2, 11 bits signés, **LSB = 0,25 °C** (`sign_extend32(reg >> 2, 10) * 250` m°C).
- Les MR 0x00..0x7F sont directement adressables : **aucune sélection de page ni écriture** (MR11 / pages NVM non utilisés).

## Octets émis (tous des lectures)
Reconnaissance : lecture d'octet 0x50+n reg 0x00, 0x01, 0x05, 0x1A (DDR5) ou 0x02 puis lecture de mot 0x18+n reg 0x05 (DDR4). Relevé : lectures 0x31/0x32 ou mot 0x05.
Le seul octet « écrit » est la commande (pointeur de registre) à l'intérieur de la transaction de lecture SMBus, indissociable d'une lecture. Aucune écriture de configuration, d'EEPROM ou de protection SPD.
Valeur hors -40..150 °C = rejetée.

## Garde-fous PureRGB
- Actif seulement si les pilotes natifs sont activés (réglage « RAM/SMBus » existant, désactivé par défaut) et PawnIO prêt ; sinon message clair dans Surveillance.
- Si un logiciel RGB qui tient le bus tourne (processus corsair/asus/msi/gigabyte/asrock/adata_xpg/signalrgb de `conflicts.rs`), aucune lecture et message nommant le logiciel.
- Relevé réel au plus toutes les 10 s (cache), emplacements mémorisés après la première détection ; mutex global pris par chaque transfert.
- Hypothèse non vérifiée : code de taille 3 (mot) du transfert PawnIO, convention Linux i2c-dev (voir `pawnio.rs`). Une erreur = lecture refusée, pas d'écriture.
