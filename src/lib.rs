//! Generador de rompecabezas Montessori a partir de una Ilustración.
//! Vocabulario en GLOSSARY.md; decisiones en docs/adr/.

pub mod bordes;
pub mod geometria;
pub mod mascara;
pub mod particion;
pub mod pestana;
pub mod rompecabezas;
pub mod svg;
pub mod vista;

/// Lo que el usuario ajusta para generar un Rompecabezas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parametros {
    pub piezas: usize,
    pub ancho_mm: f64,
    pub umbral: u8,
    pub margen_mm: f64,
    pub tamano_pestana: f64,
    pub jitter: f64,
    pub semilla: u64,
}

impl Default for Parametros {
    fn default() -> Self {
        Self {
            piezas: 5,
            ancho_mm: 200.0,
            umbral: 128,
            margen_mm: 1.0,
            tamano_pestana: 0.2,
            jitter: 0.3,
            semilla: 0,
        }
    }
}
