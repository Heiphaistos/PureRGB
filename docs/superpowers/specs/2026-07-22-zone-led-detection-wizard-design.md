# PureRGB — Assistant de détection du nombre de LEDs — Design

## Contexte

Sous-projet 5/5 (dernier de la série — 1/5 auto-update OpenRGB, 2/5 import catalogue OpenRGB, 3/5 notification hotplug, 4/5 catalogue appareils connus, tous livrés). Objectif reformulé : "auto-détection zones/LED-count au lieu du redimensionnement manuel existant".

**Note d'autonomie** (mandat "fait tout le reste") : conception validée en autonomie, documentée pour audit.

## Contrainte matérielle vérifiée — pourquoi "auto-détection" ne peut pas être 100% automatique

Le code contient déjà ce constat, écrit avant ce sous-projet (`EffectPanel.vue`, commentaire existant) : **"détection matérielle automatique impossible sur un header 3-pin passif"**. Un connecteur ARGB standard (WS2812B et dérivés) est unidirectionnel — la carte mère/hub envoie les couleurs, la bande ne renvoie jamais son nombre de LEDs. OpenRGB lui-même n'a aucun moyen de le lire. Toute solution est donc nécessairement **semi-automatique** : le logiciel doit faire clignoter/allumer un motif et demander une confirmation visuelle brève à l'utilisateur — mais on peut rendre ça bien plus rapide et fiable qu'un "redimensionnement manuel" où l'utilisateur doit déjà connaître ou deviner le nombre exact.

**Décision de conception** : un assistant qui teste par **recherche dichotomique** (log2(N) étapes, ex. ~9 clics pour une bande de 300 LEDs au lieu de deviner un nombre à l'aveugle) : à chaque étape, le logiciel allume en blanc les N premières LEDs (N = candidat courant) et éteint le reste, puis demande une seule question fermée "Oui/Non" ("Est-ce que TOUTES les LEDs de la bande sont allumées en blanc, y compris la toute dernière ?"). Bien plus rapide et fiable qu'un champ numérique à deviner, et n'exige aucune nouvelle infrastructure de pilotage LED bas niveau.

## Fait vérifié dans le code existant — aucun nouveau code backend nécessaire

- `resize_zone` (commande Tauri, `lib.rs:121-176`) : redimensionne une zone OpenRGB, **clampe déjà** sur les bornes matérielles réelles rapportées par OpenRGB (`z.leds_min`/`z.leds_max`, issues du firmware/protocole — pas des valeurs inventées), persiste automatiquement dans `settings.zone_sizes`, et déclenche un `reg.scan_all()` + `engine.invalidate()`.
- `apply_effect` (commande Tauri, `lib.rs:196-241`) : applique un `EffectConfig` à une zone précise (`zone: Some(i)`, avec `offset`/`len` calculés automatiquement à partir de la zone). Un `EffectKind::Static` avec une couleur = tout allumer d'une seule couleur unie sur la zone actuellement dimensionnée.
- **Ces deux commandes existantes suffisent entièrement** à l'algorithme de recherche dichotomique : (1) redimensionner la zone au candidat testé, (2) appliquer un Static blanc, (3) demander confirmation visuelle. Aucune nouvelle commande Rust, aucun nouveau protocole bas niveau (set de couleurs par index arbitraire) n'est nécessaire — l'assistant est **entièrement un ajout frontend** (Vue), orchestrant des appels déjà existants.
- `zoneResizable(z)` (helper déjà utilisé, `types.ts`) déterime déjà si une zone accepte un redimensionnement — réutilisé tel quel pour savoir où proposer le bouton "Assistant".
- Séquence de nettoyage nécessaire avant de démarrer la recherche : redimensionner d'abord la zone à `leds_max` (borne haute réelle du zone, pas un nombre arbitraire) et appliquer un Static **noir** sur toute cette plage — sinon d'anciennes LEDs physiques au-delà du candidat testé pourraient conserver une couleur précédente non remise à zéro, rendant la frontière blanc/noir ambiguë pour l'utilisateur.

## Portée

### `src/components/EffectPanel.vue` uniquement

- Nouvel état réactif, **séparé** de `zoneSizeEdits` (qui est réinitialisé par le `watch(() => props.device, ...)` existant — le nouvel état du wizard ne doit PAS être écrasé par ce même watcher à chaque rafraîchissement de `props.device` provoqué par les appels `resize_zone` du wizard lui-même) :
  - `wizardZone: Ref<number | null>` — index de la zone en cours de détection, `null` = aucun assistant actif.
  - `wizardLow`/`wizardHigh: Ref<number>` — bornes courantes de la recherche dichotomique.
  - `wizardOriginalSize: Ref<number | null>` — taille de la zone AVANT le lancement de l'assistant, pour pouvoir restaurer si l'utilisateur annule en cours de route.
  - `wizardBusy: Ref<boolean>` — désactive les boutons Oui/Non pendant qu'un appel `invoke` est en cours.
- Bouton "Assistant de détection" à côté du champ de saisie manuelle existant, visible uniquement pour les zones `zoneResizable(z)` (même condition que l'affichage actuel du redimensionnement manuel).
- Démarrage (`startWizard(zoneIdx)`) :
  1. Mémorise `wizardOriginalSize = zoneSizeEdits.value[zoneIdx] ?? device.zones[zoneIdx].led_count`.
  2. `wizardLow = z.leds_min`, `wizardHigh = z.leds_max`.
  3. Redimensionne à `leds_max`, applique un Static **noir** dessus (nettoyage).
  4. Lance la première étape de test.
- Étape de test (`testMid()`) : `mid = Math.ceil((low + high) / 2)`, appelle `resize_zone` avec `mid`, puis `apply_effect` Static **blanc** sur cette zone (maintenant dimensionnée à `mid`).
- Réponse utilisateur :
  - **"Oui, toutes allumées"** → `wizardLow = mid`. Si `low === high`, terminé (taille trouvée = `low`, déjà appliquée par le dernier `resize_zone`, rien de plus à faire — `zone_sizes` déjà persisté par la commande elle-même). Sinon, nouvelle étape de test.
  - **"Non, certaines éteintes à la fin"** → `wizardHigh = mid - 1`. Si `low === high`, terminé (taille trouvée = `low`, il faut re-appeler `resize_zone` une dernière fois avec cette valeur car le dernier test affiché correspondait à un candidat trop grand). Sinon, nouvelle étape de test.
- Bouton "Annuler" à tout moment pendant l'assistant : ré-appelle `resize_zone` avec `wizardOriginalSize`, ferme l'assistant (`wizardZone.value = null`).
- Fin réussie : toast "Zone « {nom} » : {N} LED détectées", `emit("refresh")` (déjà fait implicitement par chaque `resize_zone`, mais un rafraîchissement final explicite garantit que l'UI affiche l'état définitif).

## Cas limites

- Zone déjà à `leds_max === leds_min` (taille fixe, pas vraiment "redimensionnable" en pratique même si techniquement listée) : la recherche dichotomique termine immédiatement en une étape (low == high dès le départ), pas de bug, juste un assistant très court.
- Utilisateur ferme l'onglet/l'app en plein milieu de l'assistant : la zone reste à la dernière taille testée (pas de callback de nettoyage possible à la fermeture du process) — comportement acceptable, identique à ce qui se passerait avec un redimensionnement manuel interrompu de la même façon ; l'utilisateur peut relancer l'assistant ou corriger manuellement au prochain lancement.
- Erreur réseau/OpenRGB pendant un appel `resize_zone`/`apply_effect` en plein wizard (ex. serveur OpenRGB redémarre) : `catch` affiche un toast d'erreur, `wizardBusy` repasse à `false`, l'assistant reste ouvert à l'étape courante — l'utilisateur peut réessayer la même question ou annuler.
- Aucune persistance de l'état du wizard entre rechargements de page/composant (pas nécessaire — c'est une interaction ponctuelle courte, quelques secondes).

## Tests

- Aucune suite de tests JS n'existe dans ce dépôt (déjà vérifié lors du sous-projet 3/5 — `find src -iname "*.test.ts"` vide) — vérification manuelle uniquement, cohérente avec le reste du frontend de cette app.
- Vérification manuelle à faire par Momo (nécessite une vraie zone ARGB redimensionnable et du matériel réel) : lancer l'assistant sur une zone connue, répondre honnêtement aux questions Oui/Non, confirmer que la taille détectée correspond au nombre réel de LEDs physiques.

## Statut

Conception validée en autonomie, prête pour plan d'implémentation.
