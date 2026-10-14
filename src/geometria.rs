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

/// Distancia de los puntos de control al extremo para aproximar un cuarto de
/// círculo de radio 1 con una cúbica.
const KAPPA: f64 = 0.552_284_75;

/// Contorno cerrado de `r` con las esquinas redondeadas con `radio` (0 da
/// esquinas rectas), recorrido en sentido horario desde arriba a la izquierda.
/// El radio se limita a la mitad del lado menor.
pub fn rectangulo_redondeado(r: Rect, radio: f64) -> Trazo {
    let radio = radio.clamp(0.0, r.ancho.min(r.alto) / 2.0);
    let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.ancho, r.y + r.alto);
    let k = radio * KAPPA;
    // Por esquina: el vértice, la dirección del lado que llega y la del que
    // sale (vectores unitarios).
    let esquinas = [
        ((x1, y0), (1.0, 0.0), (0.0, 1.0)),
        ((x1, y1), (0.0, 1.0), (-1.0, 0.0)),
        ((x0, y1), (-1.0, 0.0), (0.0, -1.0)),
        ((x0, y0), (0.0, -1.0), (1.0, 0.0)),
    ];
    // Empieza donde termina el último arco, así el contorno cierra solo.
    let inicio = Punto::new(x0 + radio, y0);
    let mut actual = inicio;
    let mut segmentos = Vec::new();
    for ((cx, cy), (ex, ey), (sx, sy)) in esquinas {
        let llegada = Punto::new(cx - ex * radio, cy - ey * radio);
        let salida = Punto::new(cx + sx * radio, cy + sy * radio);
        // Si el radio es la mitad del lado, ese lado no tiene tramo recto.
        if llegada != actual {
            segmentos.push(Segmento::Linea(llegada));
        }
        if radio > 0.0 {
            segmentos.push(Segmento::Cubica(
                Punto::new(llegada.x + ex * k, llegada.y + ey * k),
                Punto::new(salida.x - sx * k, salida.y - sy * k),
                salida,
            ));
        }
        actual = salida;
    }
    Trazo { inicio, segmentos }
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

    fn rect() -> Rect {
        Rect {
            x: 10.0,
            y: 20.0,
            ancho: 100.0,
            alto: 60.0,
        }
    }

    #[test]
    fn rectangulo_con_esquinas_rectas_pasa_por_los_cuatro_vertices() {
        let t = rectangulo_redondeado(rect(), 0.0);
        let vertices = [(10.0, 20.0), (110.0, 20.0), (110.0, 80.0), (10.0, 80.0)];
        let puntos: Vec<Punto> = t.puntos().collect();
        assert_eq!(puntos.len(), 5);
        for (x, y) in vertices {
            assert!(puntos.contains(&Punto::new(x, y)), "falta ({x}, {y})");
        }
        assert_eq!(puntos.first(), puntos.last());
    }

    #[test]
    fn rectangulo_redondeado_sigue_los_arcos_y_cierra() {
        let r = rect();
        let radio = 8.0;
        let t = rectangulo_redondeado(r, radio);
        let p = crate::vista::aplanar(&t);
        assert_eq!(p.first(), p.last());
        let centros = [(18.0, 28.0), (102.0, 28.0), (102.0, 72.0), (18.0, 72.0)];
        for q in &p {
            assert!(q.x >= 10.0 - 1e-9 && q.x <= 110.0 + 1e-9);
            assert!(q.y >= 20.0 - 1e-9 && q.y <= 80.0 + 1e-9);
            // En la zona de una esquina, el contorno está sobre el arco.
            for (cx, cy) in centros {
                let en_esquina = (q.x - cx) * (cx - 60.0) > 0.0 && (q.y - cy) * (cy - 50.0) > 0.0;
                if en_esquina {
                    let d = (q.x - cx).hypot(q.y - cy);
                    assert!((d - radio).abs() < 0.01, "radio {d} en {q:?}");
                }
            }
        }
        let cubicas = t
            .segmentos
            .iter()
            .filter(|s| matches!(s, Segmento::Cubica(..)))
            .count();
        assert_eq!(cubicas, 4);
    }

    #[test]
    fn el_radio_se_limita_a_la_mitad_del_lado_menor() {
        // Alto 60: radio máximo 30, los lados cortos quedan sin tramo recto.
        let t = rectangulo_redondeado(rect(), 50.0);
        let p = crate::vista::aplanar(&t);
        let (y0, y1) = p
            .iter()
            .fold((f64::MAX, f64::MIN), |c, q| (c.0.min(q.y), c.1.max(q.y)));
        assert!((y0 - 20.0).abs() < 1e-9 && (y1 - 80.0).abs() < 1e-9);
        let rectas = t
            .segmentos
            .iter()
            .filter(|s| matches!(s, Segmento::Linea(_)))
            .count();
        assert_eq!(rectas, 2);
    }
}
