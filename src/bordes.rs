//! Ronda 03: grafo de bordes entre etiquetas. Ver docs/adr/0001.

use crate::geometria::Polilinea;
use crate::particion::Etiquetas;

/// Un tramo de borde entre dos etiquetas distintas, de cruce a cruce (o un
/// lazo cerrado si no toca ningún cruce). Si `a` es `FONDO`, es parte de la
/// Silueta; si no, es una Unión.
#[derive(Debug, Clone, PartialEq)]
pub struct Borde {
    /// La etiqueta menor de las dos.
    pub a: u16,
    /// La etiqueta mayor de las dos.
    pub b: u16,
    /// En píxeles de la grilla, simplificada y suavizada, con los extremos
    /// exactamente en los cruces.
    pub puntos: Polilinea,
}

/// Extrae todos los Bordes entre etiquetas distintas, incluido el fondo.
pub fn extraer_bordes(etiquetas: &Etiquetas) -> Vec<Borde> {
    let _ = etiquetas;
    todo!("ronda 03")
}
