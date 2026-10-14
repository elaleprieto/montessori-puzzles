//! Ronda 04: Pestaña tipo hongo sobre una Unión.

use crate::geometria::{Punto, Trazo};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParametrosPestana {
    /// Ancho de la Pestaña como fracción del largo de la Unión (p. ej. 0.2).
    pub tamano: f64,
    /// Deformación aleatoria, de 0.0 (todas iguales) a 1.0.
    pub jitter: f64,
}

/// Recorre la Unión (en mm) e inserta una Pestaña en su parte más recta.
/// La `semilla` decide la deformación y hacia qué lado sale la Pestaña.
/// El Trazo empieza y termina exactamente en los extremos de la Unión.
pub fn con_pestana(union: &[Punto], parametros: ParametrosPestana, semilla: u64) -> Trazo {
    let _ = (union, parametros, semilla);
    todo!("ronda 04")
}
