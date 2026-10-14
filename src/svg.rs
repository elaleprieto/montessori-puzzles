//! Ronda 05: exportación del Rompecabezas a SVG para láser.

use crate::geometria::{Rect, Trazo};
use crate::mascara::Ilustracion;

/// Todo lo que va al archivo, ya en milímetros.
#[derive(Debug, Clone)]
pub struct Dibujo<'a> {
    pub ancho_mm: f64,
    pub alto_mm: f64,
    /// La Silueta y las Uniones: van a la Capa de corte.
    pub cortes: Vec<Trazo>,
    /// Va a la Capa de grabado, ubicada en `ilustracion_en`.
    pub ilustracion: &'a Ilustracion,
    /// Dónde va la Ilustración dentro del lienzo, en mm.
    pub ilustracion_en: Rect,
}

/// Genera el SVG con las capas `Corte` y `Grabado`.
pub fn exportar(dibujo: &Dibujo) -> String {
    let _ = dibujo;
    todo!("ronda 05")
}
