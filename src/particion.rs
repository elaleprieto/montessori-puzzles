//! Ronda 02: división de la máscara en Piezas. Ver docs/adr/0001.

use crate::mascara::Mascara;

/// Etiqueta de fondo en una grilla de Etiquetas.
pub const FONDO: u16 = 0;

/// Grilla del mismo tamaño que la Máscara donde cada píxel lleva el número de
/// su Pieza (1..=N) o `FONDO`. Se indexa por fila: `datos[y * ancho + x]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Etiquetas {
    pub ancho: usize,
    pub alto: usize,
    pub datos: Vec<u16>,
}

impl Etiquetas {
    pub fn get(&self, x: usize, y: usize) -> u16 {
        self.datos[y * self.ancho + x]
    }
}

/// Divide la figura en `piezas` Piezas conexas de área pareja. La misma
/// `semilla` produce siempre la misma Variante.
pub fn particionar(mascara: &Mascara, piezas: usize, semilla: u64) -> Etiquetas {
    let _ = (mascara, piezas, semilla);
    todo!("ronda 02")
}
