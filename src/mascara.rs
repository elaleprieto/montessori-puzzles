//! Ronda 01: de la Ilustración a la máscara de la figura.

use anyhow::Result;

/// Formato de archivo de una Ilustración.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formato {
    Png,
    Jpg,
    Svg,
}

/// La imagen que carga el usuario, tal cual vino del archivo.
#[derive(Debug, Clone)]
pub struct Ilustracion {
    pub bytes: Vec<u8>,
    pub formato: Formato,
    /// Tamaño intrínseco en píxeles (para SVG, el de su viewBox).
    pub ancho_px: u32,
    pub alto_px: u32,
}

impl Ilustracion {
    /// Decodifica lo justo para validar el archivo y conocer su tamaño.
    pub fn desde_bytes(bytes: Vec<u8>, formato: Formato) -> Result<Self> {
        let _ = (bytes, formato);
        todo!("ronda 01")
    }
}

/// Rasteriza la Ilustración a RGBA con su lado mayor en `lado_max` px.
pub fn rasterizar(ilustracion: &Ilustracion, lado_max: usize) -> Result<image::RgbaImage> {
    let _ = (ilustracion, lado_max);
    todo!("ronda 01")
}

/// Grilla binaria de trabajo: `true` es figura, `false` es fondo.
/// Se indexa por fila: `datos[y * ancho + x]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mascara {
    pub ancho: usize,
    pub alto: usize,
    pub datos: Vec<bool>,
}

impl Mascara {
    pub fn vacia(ancho: usize, alto: usize) -> Self {
        Self {
            ancho,
            alto,
            datos: vec![false; ancho * alto],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> bool {
        self.datos[y * self.ancho + x]
    }

    pub fn set(&mut self, x: usize, y: usize, valor: bool) {
        self.datos[y * self.ancho + x] = valor;
    }

    /// Rasteriza la Ilustración con su lado mayor en `lado_max` px, la ubica
    /// a `relleno_px` del borde (la grilla mide la Ilustración + 2 × relleno
    /// por lado) y separa la figura del fondo según el Umbral (0–255).
    pub fn desde_ilustracion(
        ilustracion: &Ilustracion,
        umbral: u8,
        lado_max: usize,
        relleno_px: usize,
    ) -> Result<Self> {
        let _ = (ilustracion, umbral, lado_max, relleno_px);
        todo!("ronda 01")
    }

    /// Se queda con la componente conexa más grande y rellena sus huecos.
    pub fn figura_principal(&self) -> Self {
        todo!("ronda 01")
    }

    /// Dilata la figura `radio_px` píxeles (el Margen ya convertido a píxeles).
    pub fn dilatar(&self, radio_px: f64) -> Self {
        let _ = radio_px;
        todo!("ronda 01")
    }
}
