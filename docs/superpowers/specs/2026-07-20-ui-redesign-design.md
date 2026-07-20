# PureRGB — Refonte UI (esthétique SignalRGB) — Design

Sous-projet 1/4 de la refonte demandée par Momo (redesign UI, bibliothèque effets, matrice compatibilité, gestion conflits auto). Les 3 autres font l'objet de specs séparées, écrites après implémentation de celle-ci.

## Contexte

App actuelle : liste latérale texte (nom+LED count+note) + panneau détail à droite, style fonctionnel mais peu visuel. Objectif : grille de cartes périphérique façon SignalRGB (icône/logo/nom/statut/couleur visibles d'un coup d'œil), catégorisation déjà correcte côté données (21 `device_type`), juste jamais exploitée visuellement.

## Portée

100% frontend (Vue3 + TS), **zéro changement backend Rust**. Aucune vraie photo produit par modèle (infeasible : pas de SKU exact dans les noms OpenRGB, droit d'auteur sur photos fabricant, besoin offline) — remplacé par icônes vectorielles catégorie + logo marque détectée via `DeviceInfo.vendor`.

## Fichiers

- `src/style.css` — tokens étendus (rayons, ombres, espacements, transitions), palette dark #0d0d10 + accent orange #ff5000 conservée (cohérence site vitrine purergb.heiphaistos.org)
- `src/components/DeviceCard.vue` (nouveau) — carte réutilisable
- `src/components/DeviceGrid.vue` (nouveau) — remplace le mode liste dans l'onglet Éclairage
- `src/components/EffectDrawer.vue` (nouveau) — `EffectPanel` existant déplacé en tiroir latéral
- `src/assets/icons/` (nouveau) — 21 icônes SVG, une par `device_type`
- `src/assets/brands.ts` (nouveau) — table vendor → {logo, couleur}
- `App.vue` — topbar/bannières restylées, `DeviceCard` compact réutilisé dans FanPanel/LcdPanel
- `src/components/DeviceList.vue` — supprimé (remplacé par `DeviceGrid.vue`)

## Composants & data flow

**`DeviceCard.vue`**
Props : `device: DeviceInfo`, `effect: EffectConfig | undefined` (= `settings.effects[device.id]`), `selected: boolean`, `compact?: boolean`.
Affiche : icône `device_type` (map statique), logo marque si `device.vendor` matche `brands.ts` (normalisation lowercase, `includes()` fallback sur `device.name` si vendor vide), sinon chip monogramme (1re lettre, teinte dérivée du hash du nom) ; nom ; badge catégorie (`DEVICE_TYPE_LABELS`) ; swatch couleur = `effect.colors[0]` ou gris "éteint" si `kind==="off"` / pas d'effet sauvegardé ; point statut vert (connecté) / gris (`!controllable`).

**`DeviceGrid.vue`**
Reprend le regroupement par catégorie de l'actuel `DeviceList.vue` (`DEVICE_TYPE_LABELS`, tri alpha), rend des `DeviceCard` en grille CSS `grid-template-columns: repeat(auto-fill, minmax(180px, 1fr))`. Clic carte → `selectedId` + ouverture `EffectDrawer`. Re-clic même carte → ferme le drawer.

**`EffectDrawer.vue`**
Tiroir slide-in droite (420px), overlay + backdrop-blur, contient `EffectPanel` inchangé. Fermeture : Échap, clic backdrop, re-clic carte active.

**`brands.ts`**
`Record<string, {logo: Component, color: string}>`, clé = vendor normalisé. Marques couvertes au lancement : Corsair, NZXT, ASUS, MSI, Gigabyte, Razer, Logitech, EVGA, Cooler Master, Lian Li, Thermaltake, DeepCool. Extensible sans toucher aux composants.

## Cas limites

- `vendor` vide ou marque non répertoriée → chip monogramme, jamais d'icône cassée.
- 0 périphérique détecté → état vide actuel conservé tel quel.
- Fenêtre étroite → grille retombe à 1 colonne (pas de `min-width` fixe façon liste actuelle).
- `FanPanel` / `LcdPanel` : `DeviceCard` en variante `compact` (pas de drawer, comportement de sélection inchangé, juste l'habillage visuel).

## Tests

Aucune logique backend touchée → aucun nouveau test Rust requis. Vérification réelle obligatoire avant de clore : `npm run tauri dev`, contrôle visuel grille rendue, ouverture/fermeture drawer, fallback icône sur device sans vendor connu, avant/après capture.

## Statut

Approuvé par Momo (toutes sections) le 2026-07-20. Prêt pour plan d'implémentation.
