# Implémentation Lian Li + TURZX/Turing USB (branche screens/lianli)

Date : 2026-10-21. Sources : voir `lian-li.md`, `generiques-usb.md` ; clones lus hors dépôt (`sgtaziz/lian-li-linux` MIT,
`andyjusa/hydroshift2-rs` MIT, `turing-smart-screen-python` GPL lu comme description seulement, rien copié).

## Fichiers

| Fichier | Provider à enregistrer | Matériel |
|---|---|---|
| `src-tauri/src/screens/drivers/lianli_hid.rs` | `lianli_hid::LianLiHidProvider` | HID 0416:7391 / 7395 / 7398 / 7399 / 739A, 480x480 JPEG |
| `src-tauri/src/screens/drivers/turzx_usb.rs` | `turzx_usb::TurzxUsbProvider` | bulk WinUSB 1CBE : Turing 0028/0046/0050/0080/0088/0092/0123, Lian Li A021/A034/A065/A088/A092/A018/A019 |

Modifs hors fichiers : `Protocol::{LianLiHid, TurzxUsb}` ajoutés à `driver.rs` (même méthode que les autres branches ;
à fusionner avec les variantes des autres agents, et à répercuter dans `Protocol` de `src/screens/types.ts`),
dépendance `des = "=0.8.1"` (RustCrypto) dans `Cargo.toml`, deux `pub mod` dans `drivers/mod.rs`.

## Implémenté ([V] dans les sources)

- HID : rapports B (id 0x02, 1024 o, entête 11 o BE), LCD control 0x0C (mode Application à la connexion, mode réglage
  pour la luminosité), JPEG 0x0E découpé en 1013 o, qualité dégradée jusqu'à 153 600 o. Acquittement 0x0C exigé,
  acquittement d'image toléré absent (comme la référence). Rapports d'état A/0x81 ignorés.
- Bulk : enveloppe 512 o (cmd en clair 500 o, DES-CBC clé=IV `slv3tuzx`, trailer A1 1A), une écriture `entête + JPEG`,
  GetVer (réponse validée : `[0]=0x0A`, ASCII imprimable en `[8..40]`), StopPlay (0x7B) avant les images sur les PID qui le
  gèrent, sondage QueryBlock (0x7A) quand le niveau de tampon (octet 8, Lian Li) dépasse 3, luminosité 0x0E
  (Lian Li 0-100, Turing 0-102 via `pct*102/100`).
- Erreurs en français : « fermez L-Connect 3 », pilote WinUSB manquant (message commun `winusb_hint` + Zadig), écran
  resté en mode bureau (1A86:AD10..AD3F / ACD1 / ACE1) signalé quand rien d'autre n'est trouvé.
- Tests (cargo) : vecteur DES classique, entête déchiffré = octets écrits à la main, entêtes HID par valeurs manuelles,
  séquences d'init, découpe/réassemblage JPEG, bornes, écritures courtes, réponses invalides.

## Non implémenté (INCONNU ou hors périmètre écran)

- HID : rapports A (pompe, ventilateurs, température, firmware), rapports C (firmware >= 1.2/1.5), H.264, rotation
  matérielle, récupération 0x17/0x8E, attente de ~10 s après démarrage, choix d'interface par usage page.
- Bulk : H.264, PNG/calque 0x66/0x67, OLED Curve A068 (1080x2288), horloge 0x33/0x34, rotation 0x0D, cadence 0x0F,
  pompe/ventilateurs 0xFB, LED 0xFC, bascule bureau->LCD (octets `35 66 33 37 35 39 64 66` vers le CH340, décrits mais
  volontairement non envoyés), vidage des réponses en trop, dongles sans fil 1CBE:0005/0006 et hub TL 0416:7372.
- Écarts de sources signalés : la fiche donne Universal Screen 1920x480 ; le code de référence et Turing l'orientent en
  portrait natif 480x1920 (retenu). Turing 9,2" = 462x1920 (Python) mais Lian Li Vision A092 = 464x1920 (Rust).

## À valider sur matériel (aucun test réel n'a été fait)

1. HID Windows : hidapi `write` de 1024 o avec identifiant 0x02 vers un périphérique multi-rapports (la référence utilise
   rusb/hidraw) ; interface HID à choisir si plusieurs ; durée d'attente après mise sous tension.
2. Bulk : endpoints 0x01/0x81 en bulk (la référence détecte parfois de l'interrupt : `NusbTransport` n'accepte que du bulk) ;
   compléments d'entête Zéro (Turing) vs PKCS#7 (Lian Li) ; contenu des acquittements (non décrit hors GetVer) ;
   désynchronisation si l'écran envoie plusieurs réponses (la référence « flush » la file) ; calque PNG laissé par L-Connect
   qui masquerait nos images (la référence efface les deux calques à l'init) ; horloge interne affichée par-dessus.
3. SL Infinity Flex (A019) : « non testé » chez la référence. Conflit d'accès si L-Connect 3 tourne (WinUSB exclusif).
4. Tous les écrans restent `verified: false` et `resolution_confirmed: false`.
