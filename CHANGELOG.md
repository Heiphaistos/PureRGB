# Journal des modifications

## 0.21.0

- Mise à jour automatique de l'application : vérification silencieuse au lancement, bouton « Rechercher une mise à jour » dans les réglages, signature minisign vérifiée, installation silencieuse (NSIS passive) puis relance.
- Nouveau réglage « Vérifier les mises à jour automatiquement » (actif par défaut), distinct du démarrage automatique d'OpenRGB.
- Publication : la CI joint l'installeur, sa signature `.sig` et `latest.json` à la release.

## 0.20.0

- Control Flow Guard activé sur les builds Windows.
