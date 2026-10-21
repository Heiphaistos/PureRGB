# Générateur de designs d'écran

Produit des designs JSON (format `src-tauri/src/screens/layout.rs`) par combinaison
**archétype de mise en page × contenu × palette × format d'écran**, avec contrôle qualité
automatique, et branche le résultat dans l'application.

```
python tools/screens-designs/generate.py            # régénère gen/*.json + generated.rs
python tools/screens-designs/generate.py --only duo,hud --dry --report   # essai sans écrire, liste des rejets
cargo test --manifest-path src-tauri/Cargo.toml --lib screens::designs   # qualité + rendu PNG
python tools/screens-designs/contact_sheets.py      # planches-contact (une par archétype)
```

Sorties (ne jamais éditer à la main, toujours régénérer) :
- `src-tauri/src/screens/designs/gen/<id>.json`
- `src-tauri/src/screens/designs/generated.rs` (`pub const LIST: &[Builtin]`, branché dans `builtin::all()`)
- PNG de revue : `src-tauri/target/screens-preview/designs/` (test `every_builtin_design_renders_a_non_blank_preview`,
  surcharge `PURERGB_PREVIEW_DIR`) ; planches : `src-tauri/target/screens-preview/sheets/`.

Options : `--per N` palettes par (archétype, contenu, format) ; `--fpc N` formats par (archétype, contenu)
(défaut 3, départ tournant pour couvrir tous les formats). Les combinaisons rejetées par le QA sont
remplacées par la palette suivante, jamais écrites. Python 3 + Pillow (mesure de texte avec les vraies polices).

## Identifiants

`gen-<archétype>-<palette>-<format>` ; si l'archétype a plusieurs contenus : `gen-<archétype>-<contenu>-<palette>-<format>`
(ex. `gen-cartes4-disques-ocean-s480`). Uniques, `[a-z0-9-]`, 64 caractères max (vérifié par le générateur).
Catégorie affichée = `TITLE` de l'archétype ; tags = format, palette, contenu, thèmes de la palette.

## Ajouter un archétype

Créer `layouts/<nom>.py` (< 300 lignes, découvert automatiquement) :

```python
NAME, TITLE = "duo", "Duo en cartes"      # slug d'id, catégorie française
CONTENTS = ["cpu-gpu", "liquide"]          # clés de content.CONTENT (défaut ["cpu-gpu"])
FORMATS = ["s480", "w1280"]                # optionnel : formats pertinents (défaut : tous)
def accepts(p): return p.dark              # optionnel : filtre de palettes

def build(palette, fmt, content):          # -> widgets (c.finish()) ou None si non pertinent
    c = Canvas(palette, fmt)
    ms = CONTENT[content][1]
    x, y, w, h = fmt.zone                  # zone sûre (marge 8 % ; rond = carré inscrit)
    ...
    return c.finish()
```

Règles du contrat :
- **S'adapter au ratio** (`fmt.ratio`, `fmt.wide`, `fmt.strip`, `fmt.round`) ou renvoyer `None`.
- Tout texte passe par `c.text` / `c.value` : la taille est ajustée à la boîte par mesure réelle de la police.
  `group="x"` harmonise la taille de tous les textes du groupe (taille minimale du groupe) : à utiliser pour les
  libellés/valeurs répétés (cartes, lignes) afin d'éviter des tailles disparates.
- `c.value(src, fmt, ..., sample)` : `sample` = texte le plus large attendu (sert à la mesure).
- Couleurs uniquement via la palette (`p.text`, `p.text2`, `p.acc[0..2]`, `p.track`, `p.card`, `p.line`,
  `p.temp_ramp`, `p.load_ramp(i)`, `m.colors(p, i)`), police via `p.font`.
- Briques réutilisables dans `parts.py` (`stat_card`, `ring`, `row`, `mini_col`, `glow`, `corners`, `hero_list`),
  grilles dans `lib.py` (`grid`, `split`, `inset`).
- Le QA rejette : widget hors canevas (> 2 px), texte hors canevas / du cercle (formats ronds), chevauchement de
  textes, texte plus petit que `lib.min_text(fmt)` (15, ou 4 % du petit côté), > 200 widgets, ids dupliqués.
  Pas de texte en dur anglais : tout en français.
- Après ajout : `generate.py --only <nom> --report`, `cargo test ... screens::designs`, `contact_sheets.py --only <nom>`,
  **regarder les planches** (PNG) et corriger ce qui est laid.

## Ajouter une palette

Une ligne dans `palettes._ROWS` : clé (slug), libellé français, sombre ?, fond (de, vers, angle), texte, texte2,
3 accents, piste, carte (translucide `#rrggbbaa`), rampe température (froid, tiède, chaud), police
(`Inter` / `Rajdhani` / `Roboto Mono`), tags. Les contrastes (texte ≥ 4,5, texte2 ≥ 3,6, accents ≥ 3, rampe ≥ 2,2
sur les deux fonds du dégradé) sont corrigés automatiquement (éclaircis/assombris) ; `qa.palette_problems` écarte
une palette encore invalide.

## Ajouter un format

Une ligne dans `formats.FORMATS` : `Fmt(clé, libellé français, largeur, hauteur, rond?)`. Un écran rond est un canevas
carré dont le contenu reste dans le cercle inscrit (`fmt.zone`).

## Ajouter une mesure / un contenu

`content.py` : `Metric(libellé, source, format, sample, min, max, "temp"|"load", sub=(source, format, sample))` puis une
entrée de `CONTENT` (tags, liste de métriques). Clés de données stables : `cpu.{temp,load,clock,power,voltage,fan}`,
`gpu.{temp,hotspot,load,clock,mem_clock,power,vram_used_pct,vram_used_gb,fan}`, `ram.{used_pct,used_gb,free_gb,total_gb}`,
`mobo.temp`, `chipset.temp`, `vrm.temp`, `ssd.N.{temp,used_pct}`, `disk.<LETTRE>.{used_pct,free_gb,total_gb}`,
`disk.{read,write}`, `net.{up,down}`, `liquid.temp`, `pump.rpm`, `fan.N.rpm`, `uptime`, `time`, `date`.

## Pièges connus

- Rajdhani / Roboto Mono : `°C` s'écarte, `Canvas.value` n'affiche que le degré (anneau `˚` en Rajdhani).
- Le moteur réduit un texte trop large : un design « qui passe » peut être illisible ; c'est le QA Python (mesure
  réelle) qui garantit la taille, le test Rust ne fait qu'une estimation.
- `generated.rs` n'est réécrit que sans `--only`.
