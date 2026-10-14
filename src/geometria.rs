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

    /// El mismo Trazo desplazado `(dx, dy)`.
    pub fn trasladado(&self, dx: f64, dy: f64) -> Self {
        let m = |p: Punto| Punto::new(p.x + dx, p.y + dy);
        Self {
            inicio: m(self.inicio),
            segmentos: self
                .segmentos
                .iter()
                .map(|s| match *s {
                    Segmento::Linea(p) => Segmento::Linea(m(p)),
                    Segmento::Cubica(a, b, c) => Segmento::Cubica(m(a), m(b), m(c)),
                })
                .collect(),
        }
    }

    /// Todos los puntos del Trazo, incluidos los de control de las curvas.
    pub fn puntos(&self) -> impl Iterator<Item = Punto> + '_ {
        std::iter::once(self.inicio).chain(self.segmentos.iter().flat_map(|s| match *s {
            Segmento::Linea(p) => vec![p],
            Segmento::Cubica(a, b, c) => vec![a, b, c],
        }))
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

    #[test]
    fn trasladar_mueve_todos_los_puntos_incluidos_los_de_control() {
        let t = Trazo {
            inicio: Punto::new(1.0, 1.0),
            segmentos: vec![
                Segmento::Linea(Punto::new(2.0, 1.0)),
                Segmento::Cubica(
                    Punto::new(3.0, 2.0),
                    Punto::new(4.0, 2.0),
                    Punto::new(5.0, 1.0),
                ),
            ],
        };
        let m = t.trasladado(-1.0, 10.0);
        let esperado: Vec<Punto> = t
            .puntos()
            .map(|p| Punto::new(p.x - 1.0, p.y + 10.0))
            .collect();
        assert_eq!(m.puntos().collect::<Vec<_>>(), esperado);
        assert_eq!(esperado.len(), 5);
    }
}
