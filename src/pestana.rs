//! Ronda 04: Pestaña tipo hongo sobre una Unión.

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::geometria::{Punto, Segmento, Trazo, largo};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParametrosPestana {
    /// Ancho de la Pestaña como fracción del largo de la Unión (p. ej. 0.2).
    pub tamano: f64,
    /// Deformación aleatoria, de 0.0 (todas iguales) a 1.0.
    pub jitter: f64,
}

const ANCHO_MIN: f64 = 8.0;
const ANCHO_MAX: f64 = 25.0;
/// Fracción del largo de la Unión que se deja libre en cada extremo
/// (se achica si la Pestaña no entra).
const MARGEN_EXTREMOS: f64 = 0.15;
/// Variación máxima del jitter.
const VARIACION: f64 = 0.15;

// Proporciones de la Pestaña respecto del ancho `w` de la cabeza.
/// Tramo de la Unión que ocupa la Pestaña.
const VENTANA: f64 = 1.6;
/// Cuánto sobresale de la Unión.
const ALTO: f64 = 0.9;
/// Ancho del cuello.
const CUELLO: f64 = 0.55;
/// Altura del cuello (su punto más angosto), en fracción del alto.
const ALTURA_CUELLO: f64 = 0.25 / ALTO;
/// Altura del ecuador de la cabeza (su punto más ancho), en fracción del alto.
const ALTURA_ECUADOR: f64 = 0.6 / ALTO;
/// Largo de las manijas de control debajo del cuello.
const MANIJA_BASE: f64 = 0.15;
/// Largo de las manijas de control entre cuello y cabeza.
const MANIJA_CUELLO: f64 = 0.2;
/// Factor de las manijas que aproxima un cuarto de elipse con una cúbica.
const KAPPA: f64 = 0.55;

/// Recorre la Unión (en mm) e inserta una Pestaña en su parte más recta.
/// La `semilla` decide la deformación y hacia qué lado sale la Pestaña.
/// El Trazo empieza y termina exactamente en los extremos de la Unión.
pub fn con_pestana(union: &[Punto], parametros: ParametrosPestana, semilla: u64) -> Trazo {
    assert!(union.len() >= 2, "la Unión necesita al menos 2 puntos");
    let total = largo(union);
    let w = (parametros.tamano * total).clamp(ANCHO_MIN, ANCHO_MAX);
    let ventana = VENTANA * w;
    if total <= ventana {
        return Trazo::desde_polilinea(union);
    }
    let margen = (MARGEN_EXTREMOS * total).min((total - ventana) / 2.0);

    let mut acum = vec![0.0];
    for p in union.windows(2) {
        acum.push(acum.last().unwrap() + dist(p[0], p[1]));
    }
    let s = ventana_mas_recta(union, &acum, margen, total - margen, ventana);
    let a = punto_en(union, &acum, s);
    let b = punto_en(union, &acum, s + ventana);

    let mut rng = ChaCha8Rng::seed_from_u64(semilla);
    let lado = if rng.random::<bool>() { 1.0 } else { -1.0 };
    let mut jit = || (rng.random::<f64>() * 2.0 - 1.0) * VARIACION * parametros.jitter;
    let (d_centro, d_ancho, d_alto, inclinacion) = (jit(), jit(), jit(), jit());

    // Sistema local: `u` a lo largo de la cuerda a→b, `v` hacia el lado de la Pestaña.
    let largo_cuerda = dist(a, b);
    let eje = Punto::new((b.x - a.x) / largo_cuerda, (b.y - a.y) / largo_cuerda);
    let normal = Punto::new(-eje.y * lado, eje.x * lado);
    let local = |u: f64, v: f64| {
        let u = u + inclinacion * v;
        Punto::new(
            a.x + eje.x * u + normal.x * v,
            a.y + eje.y * u + normal.y * v,
        )
    };

    let centro = largo_cuerda / 2.0 * (1.0 + d_centro);
    let alto = ALTO * w * (1.0 + d_alto);
    let v_cuello = ALTURA_CUELLO * alto;
    let v_ecuador = ALTURA_ECUADOR * alto;
    let (rx, ry) = (w * (1.0 + d_ancho) / 2.0, alto - v_ecuador);
    let (xl, xr) = (centro - CUELLO * w / 2.0, centro + CUELLO * w / 2.0);
    let (mb, mc) = (MANIJA_BASE * w, MANIJA_CUELLO * w);
    // Entrada y salida tangentes a la Unión (no a la cuerda) para no dejar codos.
    let (ta, tb) = (
        tangente_en(union, &acum, s, eje),
        tangente_en(union, &acum, s + ventana, eje),
    );
    let (ha, hb) = (xl / 2.0, (largo_cuerda - xr) / 2.0);
    let entrada = Punto::new(a.x + ta.x * ha, a.y + ta.y * ha);
    let salida = Punto::new(b.x - tb.x * hb, b.y - tb.y * hb);
    let cub = |c1: Punto, c2: Punto, f: Punto| Segmento::Cubica(c1, c2, f);
    let pestana = [
        cub(entrada, local(xl, v_cuello - mb), local(xl, v_cuello)),
        cub(
            local(xl, v_cuello + mc),
            local(centro - rx, v_ecuador - mc),
            local(centro - rx, v_ecuador),
        ),
        cub(
            local(centro - rx, v_ecuador + KAPPA * ry),
            local(centro - KAPPA * rx, alto),
            local(centro, alto),
        ),
        cub(
            local(centro + KAPPA * rx, alto),
            local(centro + rx, v_ecuador + KAPPA * ry),
            local(centro + rx, v_ecuador),
        ),
        cub(
            local(centro + rx, v_ecuador - mc),
            local(xr, v_cuello + mc),
            local(xr, v_cuello),
        ),
        cub(local(xr, v_cuello - mb), salida, b),
    ];

    let previos = (1..union.len()).filter(|&i| acum[i] < s - 1e-9);
    let siguientes = (1..union.len()).filter(|&i| acum[i] > s + ventana + 1e-9);
    let segmentos = previos
        .map(|i| Segmento::Linea(union[i]))
        .chain(std::iter::once(Segmento::Linea(a)))
        .chain(pestana)
        .chain(siguientes.map(|i| Segmento::Linea(union[i])))
        .collect();
    Trazo {
        inicio: union[0],
        segmentos,
    }
}

/// Inicio (en largo acumulado) de la ventana de largo `ventana` más recta entre
/// `desde` y `hasta`: mínima desviación máxima respecto de su cuerda. En empate
/// gana la más cercana al centro de la Unión.
fn ventana_mas_recta(union: &[Punto], acum: &[f64], desde: f64, hasta: f64, ventana: f64) -> f64 {
    let holgura = hasta - desde - ventana;
    let pasos = (holgura / (ventana / 40.0)).ceil().max(1.0) as usize;
    let centro = (acum[acum.len() - 1] - ventana) / 2.0;
    let mut mejor: Option<(f64, f64)> = None;
    for i in 0..=pasos {
        let s = desde + holgura * i as f64 / pasos as f64;
        let d = desviacion(union, acum, s, s + ventana);
        let gana = mejor.is_none_or(|(md, ms)| {
            d < md - 1e-6 || ((d - md).abs() <= 1e-6 && (s - centro).abs() < (ms - centro).abs())
        });
        if gana {
            mejor = Some((d, s));
        }
    }
    mejor.unwrap().1
}

fn dist(a: Punto, b: Punto) -> f64 {
    (b.x - a.x).hypot(b.y - a.y)
}

/// Índice `i` del tramo `union[i - 1]..union[i]` que contiene el largo acumulado `s`.
fn tramo_en(union: &[Punto], acum: &[f64], s: f64) -> usize {
    acum.partition_point(|&x| x <= s).clamp(1, union.len() - 1)
}

/// Punto de la polilínea a largo acumulado `s`.
fn punto_en(union: &[Punto], acum: &[f64], s: f64) -> Punto {
    let i = tramo_en(union, acum, s);
    let (p, q) = (union[i - 1], union[i]);
    let seg = acum[i] - acum[i - 1];
    let t = if seg > 0.0 {
        (s - acum[i - 1]) / seg
    } else {
        0.0
    };
    Punto::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t)
}

/// Dirección unitaria de la polilínea a largo acumulado `s` (o `defecto` si el
/// tramo tiene largo 0).
fn tangente_en(union: &[Punto], acum: &[f64], s: f64, defecto: Punto) -> Punto {
    let i = tramo_en(union, acum, s);
    let (p, q) = (union[i - 1], union[i]);
    let l = dist(p, q);
    if l > 0.0 {
        Punto::new((q.x - p.x) / l, (q.y - p.y) / l)
    } else {
        defecto
    }
}

/// Máxima distancia de la polilínea entre `s0` y `s1` a la cuerda de ese tramo.
fn desviacion(union: &[Punto], acum: &[f64], s0: f64, s1: f64) -> f64 {
    let a = punto_en(union, acum, s0);
    let b = punto_en(union, acum, s1);
    let cuerda = dist(a, b).max(1e-12);
    let d = |p: Punto| ((b.x - a.x) * (a.y - p.y) - (a.x - p.x) * (b.y - a.y)).abs() / cuerda;
    union
        .iter()
        .zip(acum)
        .filter(|(_, s)| **s > s0 && **s < s1)
        .map(|(p, _)| d(*p))
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aplanar(t: &Trazo) -> Vec<Punto> {
        let mut pts = vec![t.inicio];
        let mut act = t.inicio;
        for s in &t.segmentos {
            match *s {
                Segmento::Linea(p) => {
                    pts.push(p);
                    act = p;
                }
                Segmento::Cubica(c1, c2, f) => {
                    for i in 1..=32 {
                        let u = i as f64 / 32.0;
                        let m = 1.0 - u;
                        let (b0, b1, b2, b3) =
                            (m * m * m, 3.0 * m * m * u, 3.0 * m * u * u, u * u * u);
                        pts.push(Punto::new(
                            b0 * act.x + b1 * c1.x + b2 * c2.x + b3 * f.x,
                            b0 * act.y + b1 * c1.y + b2 * c2.y + b3 * f.y,
                        ));
                    }
                    act = f;
                }
            }
        }
        pts
    }

    fn recta(n: f64) -> Vec<Punto> {
        (0..=(n as usize))
            .map(|i| Punto::new(i as f64, 0.0))
            .collect()
    }

    fn params(tamano: f64, jitter: f64) -> ParametrosPestana {
        ParametrosPestana { tamano, jitter }
    }

    /// La `y` (con signo) del punto más alejado del eje x.
    fn y_mas_alejado(pts: &[Punto]) -> f64 {
        pts.iter()
            .map(|p| p.y)
            .fold(0.0, |a, y| if y.abs() > a.abs() { y } else { a })
    }

    /// Ancho (en x) de la curva a la altura `h` (en valor absoluto de y).
    fn ancho_a(pts: &[Punto], h: f64) -> (f64, f64) {
        let mut xs = Vec::new();
        for p in pts.windows(2) {
            let (a, b) = (p[0].y.abs() - h, p[1].y.abs() - h);
            if (a <= 0.0) != (b <= 0.0) {
                let t = a / (a - b);
                xs.push(p[0].x + (p[1].x - p[0].x) * t);
            }
        }
        let mn = xs.iter().cloned().fold(f64::MAX, f64::min);
        let mx = xs.iter().cloned().fold(f64::MIN, f64::max);
        (mn, mx)
    }

    fn cruza(p1: Punto, p2: Punto, p3: Punto, p4: Punto) -> bool {
        let o =
            |a: Punto, b: Punto, c: Punto| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        let (d1, d2, d3, d4) = (o(p3, p4, p1), o(p3, p4, p2), o(p1, p2, p3), o(p1, p2, p4));
        d1 * d2 < 0.0 && d3 * d4 < 0.0
    }

    fn se_cruza(pts: &[Punto]) -> bool {
        (0..pts.len() - 1)
            .any(|i| (i + 2..pts.len() - 1).any(|j| cruza(pts[i], pts[i + 1], pts[j], pts[j + 1])))
    }

    fn arco(radio: f64, angulo: f64) -> Vec<Punto> {
        (0..=200)
            .map(|i| {
                let a = angulo * (i as f64 / 200.0 - 0.5);
                Punto::new(radio * a.sin(), radio - radio * a.cos())
            })
            .collect()
    }

    /// Centro e inclinación del eje de la Pestaña (medidos entre cuello y cabeza,
    /// a 6 y 10 mm, donde la curva es simétrica) y ancho máximo de la cabeza,
    /// sobre una recta a lo largo del eje x con w = 20 mm.
    fn medidas(pts: &[Punto]) -> (f64, f64, f64) {
        let medio = |h: f64| {
            let (x0, x1) = ancho_a(pts, h);
            (x0 + x1) / 2.0
        };
        let inclinacion = (medio(10.0) - medio(6.0)) / 4.0;
        let centro = medio(6.0) - inclinacion * 6.0;
        let alto = y_mas_alejado(pts).abs();
        let cabeza = (80..(alto * 10.0) as usize)
            .map(|h| {
                let (x0, x1) = ancho_a(pts, h as f64 / 10.0);
                x1 - x0
            })
            .fold(0.0, f64::max);
        (centro, inclinacion, cabeza)
    }

    #[test]
    fn extremos_exactos_y_altura_de_la_pestana() {
        let u = recta(100.0);
        let t = con_pestana(&u, params(0.2, 0.0), 3);
        assert_eq!(t.inicio, u[0]);
        let pts = aplanar(&t);
        assert_eq!(*pts.last().unwrap(), *u.last().unwrap());
        let m = y_mas_alejado(&pts);
        assert!((m.abs() - 18.0).abs() < 0.5, "altura {m}");
        let signo = m.signum();
        assert!(pts.iter().all(|p| p.y * signo >= -1e-9));
        // Fuera de la ventana [34, 66] sigue sobre la recta.
        assert!(
            pts.iter()
                .filter(|p| p.y.abs() > 1e-9)
                .all(|p| p.x > 34.0 && p.x < 66.0)
        );
    }

    #[test]
    fn fuera_de_la_pestana_la_union_avanza_sin_volver_atras() {
        let u = recta(100.0);
        let t = con_pestana(&u, params(0.2, 0.0), 3);
        let cubicas: Vec<usize> = (0..t.segmentos.len())
            .filter(|&i| matches!(t.segmentos[i], Segmento::Cubica(..)))
            .collect();
        assert_eq!(cubicas.len(), 6);
        assert_eq!(cubicas[5] - cubicas[0], 5);
        let x = |s: &Segmento| match s {
            Segmento::Linea(p) => p.x,
            Segmento::Cubica(_, _, p) => p.x,
        };
        let mut xs = vec![t.inicio.x];
        xs.extend(t.segmentos[..cubicas[0]].iter().map(x));
        xs.extend(t.segmentos[cubicas[5]..].iter().map(x));
        assert!(xs.windows(2).all(|p| p[1] > p[0]), "{xs:?}");
        // 0..=33, a (34), b (66) y 67..=100: ningún vértice de la ventana.
        assert_eq!(xs.len(), 70);
    }

    #[test]
    fn forma_de_hongo_cabeza_mas_ancha_que_cuello() {
        let pts = aplanar(&con_pestana(&recta(100.0), params(0.2, 0.0), 1));
        let (n0, n1) = ancho_a(&pts, 0.26 * 20.0);
        let (h0, h1) = ancho_a(&pts, 0.6 * 20.0);
        assert!(
            (h1 - h0) > 1.5 * (n1 - n0),
            "cuello {} cabeza {}",
            n1 - n0,
            h1 - h0
        );
        assert!((h1 - h0 - 20.0).abs() < 0.5);
    }

    #[test]
    fn la_semilla_elige_el_lado() {
        let u = recta(100.0);
        let lados: Vec<f64> = (0..20)
            .map(|s| y_mas_alejado(&aplanar(&con_pestana(&u, params(0.2, 0.5), s))).signum())
            .collect();
        assert!(lados.contains(&1.0) && lados.contains(&-1.0));
    }

    #[test]
    fn jitter_cero_da_forma_simetrica_e_igual_y_jitter_uno_varia() {
        let u = recta(100.0);
        let mismo_lado: Vec<u64> = (0..20)
            .filter(|&s| y_mas_alejado(&aplanar(&con_pestana(&u, params(0.2, 0.0), s))) > 0.0)
            .collect();
        assert!(mismo_lado.len() >= 2);
        let a = con_pestana(&u, params(0.2, 0.0), mismo_lado[0]);
        let b = con_pestana(&u, params(0.2, 0.0), mismo_lado[1]);
        assert_eq!(a, b);
        let (x0, x1) = ancho_a(&aplanar(&a), 14.0);
        assert!(((x0 + x1) / 2.0 - 50.0).abs() < 0.1);

        let a = con_pestana(&u, params(0.2, 1.0), mismo_lado[0]);
        let b = con_pestana(&u, params(0.2, 1.0), mismo_lado[1]);
        assert_ne!(a, b);
    }

    #[test]
    fn el_jitter_mueve_el_centro_el_ancho_y_la_inclinacion() {
        let u = recta(100.0);
        let (centro, inclinacion, cabeza) =
            medidas(&aplanar(&con_pestana(&u, params(0.2, 0.0), 0)));
        assert!((centro - 50.0).abs() < 1e-6, "{centro}");
        assert!(inclinacion.abs() < 1e-6, "{inclinacion}");
        assert!((cabeza - 20.0).abs() < 0.1, "{cabeza}");

        let m: Vec<(f64, f64, f64)> = (0..20)
            .map(|s| medidas(&aplanar(&con_pestana(&u, params(0.2, 1.0), s))))
            .collect();
        let rango = |f: fn(&(f64, f64, f64)) -> f64| {
            let v: Vec<f64> = m.iter().map(f).collect();
            let mn = v.iter().cloned().fold(f64::MAX, f64::min);
            let mx = v.iter().cloned().fold(f64::MIN, f64::max);
            (mn, mx)
        };
        // Hasta ±15%: centro ±2.4 mm (de la mitad de la cuerda de 32 mm),
        // cabeza 17–23 mm, inclinación ±0.15.
        let (c0, c1) = rango(|m| m.0);
        assert!(
            c1 - c0 > 1.5 && c0 > 50.0 - 2.5 && c1 < 50.0 + 2.5,
            "{c0} {c1}"
        );
        let (w0, w1) = rango(|m| m.2);
        assert!(w1 - w0 > 1.5 && w0 > 16.9 && w1 < 23.1, "{w0} {w1}");
        let (i0, i1) = rango(|m| m.1);
        assert!(i1 - i0 > 0.1 && i0 > -0.16 && i1 < 0.16, "{i0} {i1}");
    }

    #[test]
    fn la_pestana_cae_en_el_tramo_recto() {
        let mut u = recta(100.0);
        for i in 1..=60 {
            let a = i as f64 / 60.0 * std::f64::consts::FRAC_PI_2;
            u.push(Punto::new(100.0 + 30.0 * a.sin(), 30.0 - 30.0 * a.cos()));
        }
        let pts = aplanar(&con_pestana(&u, params(0.2, 0.0), 2));
        let cima = pts
            .iter()
            .filter(|p| p.x < 100.0)
            .map(|p| p.y.abs())
            .fold(0.0, f64::max);
        assert!(cima > 20.0, "la Pestaña no está en el tramo recto: {cima}");
    }

    #[test]
    fn no_se_cruza_consigo_misma() {
        let u = recta(100.0);
        for &tamano in &[0.1, 0.15, 0.2, 0.25, 0.35] {
            for &jitter in &[0.0, 0.5, 1.0] {
                for semilla in 0..10 {
                    let pts = aplanar(&con_pestana(&u, params(tamano, jitter), semilla));
                    assert!(
                        !se_cruza(&pts),
                        "cruce tamano {tamano} jitter {jitter} semilla {semilla}"
                    );
                }
            }
        }
    }

    #[test]
    fn no_se_cruza_con_el_resto_de_una_union_curva() {
        use std::f64::consts::PI;
        for (radio, angulo) in [(35.0, PI), (25.0, 1.5 * PI)] {
            let u = arco(radio, angulo);
            for &tamano in &[0.1, 0.2, 0.35] {
                for &jitter in &[0.0, 1.0] {
                    for semilla in 0..6 {
                        let pts = aplanar(&con_pestana(&u, params(tamano, jitter), semilla));
                        assert!(
                            !se_cruza(&pts),
                            "cruce radio {radio} tamano {tamano} jitter {jitter} semilla {semilla}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn entra_y_sale_tangente_a_una_union_curva() {
        let u = arco(30.0, std::f64::consts::PI);
        let t = con_pestana(&u, params(0.2, 0.0), 0);
        let mut previo = t.inicio;
        let mut direccion: Option<Punto> = None;
        for s in &t.segmentos {
            let (entra, sale, fin) = match *s {
                Segmento::Linea(q) => (q, previo, q),
                Segmento::Cubica(c1, c2, q) => (c1, c2, q),
            };
            let d_in = Punto::new(entra.x - previo.x, entra.y - previo.y);
            if let Some(d) = direccion {
                let angulo = (d.x * d_in.y - d.y * d_in.x).atan2(d.x * d_in.x + d.y * d_in.y);
                // Cada vértice del arco gira 0.9°; la Pestaña no debe sumar codos.
                assert!(angulo.abs().to_degrees() < 1.0, "codo de {angulo} rad");
            }
            direccion = Some(Punto::new(fin.x - sale.x, fin.y - sale.y));
            previo = fin;
        }
    }

    #[test]
    fn una_union_de_15_mm_lleva_pestana() {
        let u = recta(15.0);
        for &tamano in &[0.1, 0.2, 0.35] {
            for &jitter in &[0.0, 1.0] {
                for semilla in 0..6 {
                    let t = con_pestana(&u, params(tamano, jitter), semilla);
                    assert!(
                        t.segmentos
                            .iter()
                            .any(|s| matches!(s, Segmento::Cubica(..)))
                    );
                    let pts = aplanar(&t);
                    assert_eq!(*pts.last().unwrap(), *u.last().unwrap());
                    assert!(pts.iter().all(|p| p.x >= 0.0 && p.x <= 15.0));
                    assert!(
                        !se_cruza(&pts),
                        "tamano {tamano} jitter {jitter} semilla {semilla}"
                    );
                }
            }
        }
    }

    #[test]
    fn es_determinista() {
        let u = recta(100.0);
        assert_eq!(
            con_pestana(&u, params(0.2, 0.7), 9),
            con_pestana(&u, params(0.2, 0.7), 9)
        );
    }

    #[test]
    fn el_ancho_se_acota() {
        let m = y_mas_alejado(&aplanar(&con_pestana(&recta(300.0), params(0.2, 0.0), 1)));
        assert!((m.abs() - 22.5).abs() < 0.5, "{m}");
        let m = y_mas_alejado(&aplanar(&con_pestana(&recta(20.0), params(0.2, 0.0), 1)));
        assert!((m.abs() - 7.2).abs() < 0.5, "{m}");
    }

    #[test]
    fn union_demasiado_corta_queda_sin_pestana() {
        let u = recta(8.0);
        assert_eq!(
            con_pestana(&u, params(0.2, 0.0), 1),
            Trazo::desde_polilinea(&u)
        );
    }

    #[test]
    #[should_panic]
    fn un_solo_punto_es_panic() {
        con_pestana(&[Punto::new(0.0, 0.0)], params(0.2, 0.0), 1);
    }

    #[test]
    fn respeta_el_margen_de_los_extremos() {
        // Recta de 40 mm en un extremo y onda en el resto.
        let mut u = recta(40.0);
        for i in 1..=100 {
            let x = 40.0 + i as f64;
            u.push(Punto::new(x, 10.0 * (x / 8.0).sin()));
        }
        let total = largo(&u);
        let mut inversa = u.clone();
        inversa.reverse();
        for union in [u, inversa] {
            let pts = aplanar(&con_pestana(&union, params(0.2, 0.0), 1));
            let min_x = pts
                .iter()
                .filter(|p| p.x < 40.0 && p.y.abs() > 1e-6)
                .map(|p| p.x)
                .fold(f64::MAX, f64::min);
            assert!(min_x >= 0.15 * total - 1.0, "min_x {min_x} total {total}");
        }
    }
}
