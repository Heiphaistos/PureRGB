# Manettes : détection et éclairage (Sony DualSense / DualShock 4)

Document d'interopérabilité. Aucun code copié : le driver Linux cité est GPL, seuls des faits de format sont repris.

## Sources

- Linux `drivers/hid/hid-playstation.c` (branche master, consulté le 2026-10-25) : https://github.com/torvalds/linux/blob/master/drivers/hid/hid-playstation.c : structures de sortie DualSense et DualShock 4, indicateurs de validité, CRC.
- Linux `drivers/hid/hid-ids.h` (même date) : VID Sony 0x054C ; PID DualShock 4 0x05C4 et 0x09CC, récepteur 0x0BA0, DualSense 0x0CE6, DualSense Edge 0x0DF2.
- hidapi 2.6.7 (`windows_native/device_info.rs`) : Windows expose les manettes XUSB (XInput, jetons `IG_`) comme périphériques HID ; le bus (USB / Bluetooth) est lu dans les identifiants compatibles du parent.

## DualSense (054C:0CE6, 054C:0DF2) : `native/dualsense.rs`

Bloc commun de 47 octets (offsets dans le bloc) : 0 valid_flag0, 1 valid_flag1, 2 moteur droit, 3 moteur gauche, 4 casque, 5 haut-parleur, 6 micro, 7 audio_control, 8 LED micro, 9 économie d'énergie, 10..36 réservé, 37 audio_control2, 38 valid_flag2, 39..40 réservé, 41 lightbar_setup, 42 led_brightness, 43 player_leds, 44 rouge, 45 vert, 46 bleu.

- USB : rapport 0x02 + bloc + 15 octets = 63 octets, sans CRC.
- Bluetooth : rapport 0x31, octet 1 = (séquence modulo 16) << 4, octet 2 = 0x10 (« tag », sens inconnu selon le driver), bloc aux octets 3..49, 24 réservés, CRC32 little-endian aux octets 74..77 = 78 octets.
- CRC : CRC-32 IEEE (init 0xFFFFFFFF, polynôme réfléchi 0xEDB88320, complément final) de [0xA2] suivi des 74 premiers octets du rapport.
- valid_flag1 : bit 2 barre lumineuse, bit 4 LED de joueur (bit 3 RELEASE_LEDS non utilisé). Envoyé : 0x14, valid_flag0 = 0, valid_flag2 = 0 : moteurs, audio, gâchettes non touchés.
- LED de joueur : 5 LED, bit i = LED i de gauche à droite (joueur 1 = 0x04 = LED du milieu). Les LED sont tout ou rien : une zone de 5 « couleurs », allumée si max(R,V,B) >= 128.
- Écriture directe, non persistante (aucun rapport de fonctionnalité d'écriture, aucun profil).

Non établi : le driver ne dit pas qu'une lecture du rapport de fonctionnalité 0x05 est nécessaire pour activer le mode complet en Bluetooth ; PureRGB la lit quand même (lecture seule, comme le fait le driver à l'initialisation) et ignore son échec. L'acceptation du rapport 0x31 par un pad resté en mode simple n'est pas vérifiable sans matériel.

## DualShock 4 (054C:05C4, 054C:09CC) : `native/dualshock4.rs`

Bloc commun de 10 octets : 0 valid_flag0 (bit 0 moteurs, bit 1 barre, bit 2 clignotement), 1 valid_flag1, 2 réservé, 3 moteur droit, 4 moteur gauche, 5 rouge, 6 vert, 7 bleu, 8 clignotement allumé, 9 clignotement éteint.

- USB : rapport 0x05 + bloc + 21 octets = 32 octets, sans CRC.
- Bluetooth : rapport 0x11, octet 1 = 0x80 (HID) | 0x40 (CRC32) | intervalle d'interrogation (4 par défaut dans le driver) = 0xC4, octet 2 = audio_control 0, bloc aux octets 3..12, 61 réservés, CRC32 aux octets 74..77 (même calcul, graine 0xA2) = 78 octets.
- Envoyé : valid_flag0 = 0x02 (barre seule).
- Le récepteur USB 054C:0BA0 n'est pas revendiqué (comportement via le récepteur non établi).

## Détection (toutes manettes) : `backends/hid/gamepads.rs`

Toute collection HID d'usage Generic Desktop (0x01) / Joystick (0x04) ou Gamepad (0x05) est listée comme « Manette ». Sans driver : non pilotable, avec la raison dans la note (Xbox : pas de RGB ; Nintendo Switch Pro : LED de joueur seulement, non pilotées, handshake USB non vérifié ; autres : aucun éclairage connu). L'identification de la Flydigi se fait sur la chaîne produit (« Flydigi » ou « Vader »), jamais sur le VID:PID seul, car elle emprunte l'ID Xbox 360 (045E:028E).

À vérifier sur la tour : que la chaîne produit vue par hidapi pour la Vader 4 contient bien « Flydigi » (le nom « Controller (Flydigi VADER4) » vient de Windows ; sa provenance exacte, HID ou DirectInput, n'est pas établie). Si hidapi ne la voit pas, elle n'est pas listée.

## Non implémenté (volontairement)

Gâchettes adaptatives, rumble, audio, mise à jour du firmware, profils : hors périmètre et risqués. Switch Pro : LED de joueur via sous-commande 0x30 documentées par la communauté, non retenues (pas de RGB, séquence d'initialisation USB non vérifiée).
