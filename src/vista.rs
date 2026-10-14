//! Ronda 07: lo que la vista previa de la app necesita del Rompecabezas, sin
//! depender de la UI: colores de Pieza, Trazos aplanados y encuadre.

use crate::geometria::{Punto, Rect, Segmento, Trazo};
use crate::particion::{Etiquetas, FONDO};

/// Tramos rectos con que se aproxima cada curva Bézier al dibujarla.
const PASOS_POR_CUBICA: usize = 16;

/// Colores de Pieza, distinguibles entre sí y con el rojo de la Capa de corte.
const PALETA: [[u8; 3]; 12] = [
    [66, 133, 244],
    [52, 168, 83],
    [251, 188, 5],
    [171, 71, 188],
    [0, 172, 193],
    [255, 112, 67],
    [124, 179, 66],
    [92, 107, 192],
    [240, 98, 146],
    [141, 110, 99],
    [38, 166, 154],
    [255, 167, 38],
];

/// Color RGBA de la Pieza `etiqueta` con la opacidad `alfa`; el fondo es transparente.
pub fn color_de_pieza(etiqueta: u16, alfa: u8) -> [u8; 4] {
    if etiqueta == FONDO {
        return [0; 4];
    }
    let [r, g, b] = PALETA[(etiqueta as usize - 1) % PALETA.len()];
    [r, g, b, alfa]
}

/// Imagen RGBA de las Piezas, un píxel por píxel de la grilla de trabajo.
pub fn imagen_de_piezas(etiquetas: &Etiquetas, alfa: u8) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(etiquetas.ancho as u32, etiquetas.alto as u32);
    for (px, &e) in img.pixels_mut().zip(&etiquetas.datos) {
        *px = image::Rgba(color_de_pieza(e, alfa));
    }
    img
}

/// Los puntos de un Trazo, con cada curva Bézier aproximada por tramos rectos.
pub fn aplanar(trazo: &Trazo) -> Vec<Punto> {
    let mut puntos = vec![trazo.inicio];
    let mut actual = trazo.inicio;
    for segmento in &trazo.segmentos {
        match *segmento {
            Segmento::Linea(p) => {
                puntos.push(p);
                actual = p;
            }
            Segmento::Cubica(c1, c2, p) => {
                for i in 1..=PASOS_POR_CUBICA {
                    let t = i as f64 / PASOS_POR_CUBICA as f64;
                    puntos.push(bezier(actual, c1, c2, p, t));
                }
                actual = p;
            }
        }
    }
    puntos
}

fn bezier(p0: Punto, c1: Punto, c2: Punto, p1: Punto, t: f64) -> Punto {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Punto::new(
        a * p0.x + b * c1.x + c * c2.x + d * p1.x,
        a * p0.y + b * c1.y + c * c2.y + d * p1.y,
    )
}

/// Dónde dibujar un lienzo de `lienzo_mm` dentro de `area` (en puntos de
/// pantalla): lo más grande posible sin deformarlo y centrado. Devuelve el
/// rectángulo en pantalla y los puntos por milímetro.
pub fn encuadrar(lienzo_mm: (f64, f64), area: Rect) -> (Rect, f64) {
    let escala = (area.ancho / lienzo_mm.0).min(area.alto / lienzo_mm.1);
    let (ancho, alto) = (lienzo_mm.0 * escala, lienzo_mm.1 * escala);
    let rect = Rect {
        x: area.x + (area.ancho - ancho) / 2.0,
        y: area.y + (area.alto - alto) / 2.0,
        ancho,
        alto,
    };
    (rect, escala)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_fondo_es_transparente_y_las_piezas_no() {
        assert_eq!(color_de_pieza(FONDO, 200)[3], 0);
        for e in 1..=12 {
            assert_eq!(color_de_pieza(e, 200)[3], 200);
        }
    }

    #[test]
    fn hasta_doce_piezas_tienen_colores_distintos_y_ninguna_es_rojo_de_corte() {
        let colores: Vec<[u8; 4]> = (1..=12).map(|e| color_de_pieza(e, 255)).collect();
        for (i, a) in colores.iter().enumerate() {
            assert_ne!(a[..3], [255, 0, 0]);
            assert!(colores[i + 1..].iter().all(|b| a != b));
        }
    }

    #[test]
    fn la_imagen_de_piezas_sigue_la_grilla() {
        // Fila de arriba: fondo, Pieza 1; fila de abajo: Pieza 2, Pieza 1.
        let et = Etiquetas {
            ancho: 2,
            alto: 2,
            datos: vec![FONDO, 1, 2, 1],
        };
        let img = imagen_de_piezas(&et, 100);
        assert_eq!(img.dimensions(), (2, 2));
        assert_eq!(img.get_pixel(0, 0).0, [0; 4]);
        assert_eq!(img.get_pixel(1, 0).0, color_de_pieza(1, 100));
        assert_eq!(img.get_pixel(0, 1).0, color_de_pieza(2, 100));
        assert_eq!(img.get_pixel(1, 1), img.get_pixel(1, 0));
    }

    #[test]
    fn aplanar_respeta_las_rectas() {
        let pts = [
            Punto::new(0.0, 0.0),
            Punto::new(3.0, 0.0),
            Punto::new(3.0, 4.0),
        ];
        assert_eq!(aplanar(&Trazo::desde_polilinea(&pts)), pts);
    }

    #[test]
    fn aplanar_una_cubica_pasa_por_sus_extremos_y_su_punto_medio() {
        // Arco simétrico de (0,0) a (4,0) con controles a altura 4: en t=1/2 vale (2,3).
        let t = Trazo {
            inicio: Punto::new(0.0, 0.0),
            segmentos: vec![Segmento::Cubica(
                Punto::new(0.0, 4.0),
                Punto::new(4.0, 4.0),
                Punto::new(4.0, 0.0),
            )],
        };
        let pts = aplanar(&t);
        assert_eq!(pts.len(), PASOS_POR_CUBICA + 1);
        assert_eq!(pts[0], Punto::new(0.0, 0.0));
        assert_eq!(*pts.last().unwrap(), Punto::new(4.0, 0.0));
        assert_eq!(pts[PASOS_POR_CUBICA / 2], Punto::new(2.0, 3.0));
    }

    #[test]
    fn la_cubica_continua_desde_el_final_del_tramo_anterior() {
        // Una cubica "recta" de (2,0) a (2,6) con controles sobre la misma recta.
        let t = Trazo {
            inicio: Punto::new(0.0, 0.0),
            segmentos: vec![
                Segmento::Linea(Punto::new(2.0, 0.0)),
                Segmento::Cubica(
                    Punto::new(2.0, 2.0),
                    Punto::new(2.0, 4.0),
                    Punto::new(2.0, 6.0),
                ),
            ],
        };
        assert!(aplanar(&t)[2..].iter().all(|p| p.x == 2.0));
    }

    #[test]
    fn encuadrar_mantiene_la_proporcion_y_centra() {
        let area = Rect {
            x: 10.0,
            y: 20.0,
            ancho: 400.0,
            alto: 100.0,
        };
        // Un lienzo de 100×50 mm entra a 2 puntos/mm, limitado por el alto.
        let (r, escala) = encuadrar((100.0, 50.0), area);
        assert_eq!(escala, 2.0);
        assert_eq!(
            r,
            Rect {
                x: 110.0,
                y: 20.0,
                ancho: 200.0,
                alto: 100.0,
            }
        );
    }
}
