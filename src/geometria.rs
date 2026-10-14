//! Tipos geométricos compartidos por todos los módulos.

/// Un punto en el plano. Las unidades dependen del contexto: píxeles de la
/// grilla de trabajo o milímetros del Rompecabezas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Punto {
    pub x: f64,
    pub y: f64,
}

impl Punto {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Una secuencia de puntos unidos por segmentos rectos.
pub type Polilinea = Vec<Punto>;

/// Largo total de una Polilínea.
pub fn largo(puntos: &[Punto]) -> f64 {
    puntos
        .windows(2)
        .map(|p| (p[1].x - p[0].x).hypot(p[1].y - p[0].y))
        .sum()
}

/// Un rectángulo alineado a los ejes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub ancho: f64,
    pub alto: f64,
}

/// Un tramo de un Trazo, que empieza donde terminó el tramo anterior.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segmento {
    Linea(Punto),
    /// Curva Bézier cúbica: dos puntos de control y el punto final.
    Cubica(Punto, Punto, Punto),
}

/// Un camino continuo de rectas y curvas, listo para exportar como `<path>`.
#[derive(Debug, Clone, PartialEq)]
pub struct Trazo {
    pub inicio: Punto,
    pub segmentos: Vec<Segmento>,
}

impl Trazo {
    /// Un Trazo de rectas que recorre la Polilínea. Panics si está vacía.
    pub fn desde_polilinea(puntos: &[Punto]) -> Self {
        let (inicio, resto) = puntos.split_first().expect("polilínea vacía");
        Self {
            inicio: *inicio,
            segmentos: resto.iter().copied().map(Segmento::Linea).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn largo_suma_los_segmentos() {
        let l = [
            Punto::new(0.0, 0.0),
            Punto::new(3.0, 4.0),
            Punto::new(3.0, 6.0),
        ];
        assert_eq!(largo(&l), 7.0);
        assert_eq!(largo(&l[..1]), 0.0);
    }
}
