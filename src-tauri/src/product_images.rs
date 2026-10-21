//! Photos de produits : catalogue embarqué (`assets/product_catalog.json`), identification précise
//! d'un appareil (USB exact, sinon nom spécifique — jamais le seul fabricant) et téléchargement
//! sécurisé de la photo (liste blanche d'hôtes, image décodée et ré-encodée, cache disque).
//! L'application tourne en administrateur : le fichier reçu n'est JAMAIS exécuté ni servi tel quel,
//! seul le PNG produit par nous (chemin = hash de l'URL) est lu.

mod catalog;
mod fetch;
mod imgproc;

pub use catalog::{extract_usb, identify, product_by_id, Kind};
pub use fetch::fetch_image;
