//! Ronda 03: grafo de bordes entre etiquetas. Ver docs/adr/0001.

use crate::geometria::{Polilinea, Punto};
use crate::particion::{Etiquetas, FONDO};

/// Un tramo de borde entre dos etiquetas distintas, de cruce a cruce (o un
/// lazo cerrado si no toca ningún cruce). Si `a` es `FONDO`, es parte de la
/// Silueta; si no, es una Unión.
#[derive(Debug, Clone, PartialEq)]
pub struct Borde {
    /// La etiqueta menor de las dos.
    pub a: u16,
    /// La etiqueta mayor de las dos.
    pub b: u16,
    /// En píxeles de la grilla, simplificada y suavizada, con los extremos
    /// exactamente en los cruces.
    pub puntos: Polilinea,
}

/// Tolerancia de Douglas–Peucker, en píxeles.
const TOLERANCIA: f64 = 1.0;
/// Iteraciones de suavizado Chaikin.
const ITERACIONES: usize = 3;

/// Esquina de la retícula.
type Esquina = (i64, i64);

struct Grafo<'a> {
    etiquetas: &'a Etiquetas,
}

impl Grafo<'_> {
    /// Etiqueta del píxel (x, y); fuera de la grilla es `FONDO`.
    fn etiqueta(&self, x: i64, y: i64) -> u16 {
        if x < 0 || y < 0 || x >= self.etiquetas.ancho as i64 || y >= self.etiquetas.alto as i64 {
            FONDO
        } else {
            self.etiquetas.get(x as usize, y as usize)
        }
    }

    /// Id de la arista que sale de la esquina `(x, y)` hacia la derecha
    /// (`dir` 0) o hacia abajo (`dir` 1), si separa dos etiquetas distintas.
    fn arista(&self, x: i64, y: i64, dir: usize) -> Option<usize> {
        let (a, b) = if dir == 0 {
            (self.etiqueta(x, y - 1), self.etiqueta(x, y))
        } else {
            (self.etiqueta(x - 1, y), self.etiqueta(x, y))
        };
        if a == b || x < 0 || y < 0 {
            return None;
        }
        Some(((y as usize) * (self.etiquetas.ancho + 1) + x as usize) * 2 + dir)
    }

    /// Aristas de borde que tocan la esquina: (esquina vecina, id).
    fn aristas_en(&self, (x, y): Esquina) -> Vec<(Esquina, usize)> {
        let mut r = Vec::with_capacity(4);
        if let Some(id) = self.arista(x, y, 0) {
            r.push(((x + 1, y), id));
        }
        if let Some(id) = self.arista(x, y, 1) {
            r.push(((x, y + 1), id));
        }
        if let Some(id) = self.arista(x - 1, y, 0) {
            r.push(((x - 1, y), id));
        }
        if let Some(id) = self.arista(x, y - 1, 1) {
            r.push(((x, y - 1), id));
        }
        r
    }

    fn es_cruce(&self, (x, y): Esquina) -> bool {
        let arriba_izq = self.etiqueta(x - 1, y - 1);
        let arriba_der = self.etiqueta(x, y - 1);
        let abajo_izq = self.etiqueta(x - 1, y);
        let abajo_der = self.etiqueta(x, y);
        let mut vecinas = [arriba_izq, arriba_der, abajo_izq, abajo_der];
        vecinas.sort_unstable();
        let distintas = 1 + vecinas.windows(2).filter(|par| par[0] != par[1]).count();
        distintas >= 3 || (distintas == 2 && arriba_izq == abajo_der && arriba_der == abajo_izq)
    }

    /// Par de etiquetas (menor, mayor) que separa una arista que sale de `c`
    /// hacia `n`.
    fn par(&self, c: Esquina, n: Esquina) -> (u16, u16) {
        let (a, b) = if c.1 == n.1 {
            let x = c.0.min(n.0);
            (self.etiqueta(x, c.1 - 1), self.etiqueta(x, c.1))
        } else {
            let y = c.1.min(n.1);
            (self.etiqueta(c.0 - 1, y), self.etiqueta(c.0, y))
        };
        (a.min(b), a.max(b))
    }

    /// Sigue desde `actual` por la otra arista de borde (distinta de
    /// `ultima`), la marca como visitada y devuelve (esquina siguiente, id).
    /// Panics si la arista ya estaba visitada: con un grafo bien formado no
    /// pasa nunca, y así una invariante rota no se convierte en un bucle
    /// infinito.
    fn avanzar(&self, actual: Esquina, ultima: usize, visitada: &mut [bool]) -> (Esquina, usize) {
        let (siguiente, id) = self
            .aristas_en(actual)
            .into_iter()
            .find(|&(_, i)| i != ultima)
            .expect("esquina sin cruce con una sola arista");
        assert!(
            !std::mem::replace(&mut visitada[id], true),
            "bordes: arista revisitada"
        );
        (siguiente, id)
    }
}

fn punto(c: Esquina) -> Punto {
    Punto::new(c.0 as f64, c.1 as f64)
}

/// Extrae todos los Bordes entre etiquetas distintas, incluido el fondo.
pub fn extraer_bordes(etiquetas: &Etiquetas) -> Vec<Borde> {
    let grafo = Grafo { etiquetas };
    let (w, h) = (etiquetas.ancho as i64, etiquetas.alto as i64);
    let mut visitada = vec![false; (etiquetas.ancho + 1) * (etiquetas.alto + 1) * 2];
    let mut bordes = Vec::new();

    // Tramos que salen de cada cruce, hasta el siguiente cruce.
    for y in 0..=h {
        for x in 0..=w {
            let inicio = (x, y);
            if !grafo.es_cruce(inicio) {
                continue;
            }
            for (n, id) in grafo.aristas_en(inicio) {
                if visitada[id] {
                    continue;
                }
                visitada[id] = true;
                let par = grafo.par(inicio, n);
                let mut camino = vec![inicio, n];
                let (mut actual, mut ultima) = (n, id);
                while !grafo.es_cruce(actual) {
                    (actual, ultima) = grafo.avanzar(actual, ultima, &mut visitada);
                    camino.push(actual);
                }
                let puntos: Vec<Punto> = camino.into_iter().map(punto).collect();
                bordes.push(Borde {
                    a: par.0,
                    b: par.1,
                    puntos: suavizar_abierta(&puntos),
                });
            }
        }
    }

    // Lo que queda sin visitar son lazos sin cruces.
    for y in 0..=h {
        for x in 0..=w {
            let inicio = (x, y);
            for (n, id) in grafo.aristas_en(inicio) {
                if visitada[id] {
                    continue;
                }
                visitada[id] = true;
                let par = grafo.par(inicio, n);
                let mut camino = vec![inicio];
                let (mut actual, mut ultima) = (n, id);
                while actual != inicio {
                    camino.push(actual);
                    (actual, ultima) = grafo.avanzar(actual, ultima, &mut visitada);
                }
                let anillo: Vec<Punto> = camino.into_iter().map(punto).collect();
                bordes.push(Borde {
                    a: par.0,
                    b: par.1,
                    puntos: suavizar_cerrada(&anillo),
                });
            }
        }
    }

    bordes.sort_by(|p, q| {
        (p.a, p.b).cmp(&(q.a, q.b)).then_with(|| {
            p.puntos
                .iter()
                .zip(&q.puntos)
                .map(|(u, v)| u.x.total_cmp(&v.x).then(u.y.total_cmp(&v.y)))
                .find(|o| o.is_ne())
                .unwrap_or(p.puntos.len().cmp(&q.puntos.len()))
        })
    });
    bordes
}

fn dist_segmento(p: Punto, a: Punto, b: Punto) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    if l2 == 0.0 {
        return (p.x - a.x).hypot(p.y - a.y);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0);
    (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
}

/// Douglas–Peucker: marca en `keep` los puntos que sobreviven.
fn douglas_peucker(p: &[Punto], keep: &mut [bool]) {
    let mut pila = vec![(0, p.len() - 1)];
    while let Some((i, j)) = pila.pop() {
        let mut mejor = (0.0, 0);
        for k in i + 1..j {
            let d = dist_segmento(p[k], p[i], p[j]);
            if d > mejor.0 {
                mejor = (d, k);
            }
        }
        if mejor.0 > TOLERANCIA {
            keep[mejor.1] = true;
            pila.push((i, mejor.1));
            pila.push((mejor.1, j));
        }
    }
}

fn simplificar(p: &[Punto]) -> Vec<Punto> {
    if p.len() < 3 {
        return p.to_vec();
    }
    let mut keep = vec![false; p.len()];
    keep[0] = true;
    keep[p.len() - 1] = true;
    douglas_peucker(p, &mut keep);
    p.iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(q, _)| *q)
        .collect()
}

/// Corte de esquinas: a lo sumo 2 px (o 1/4 del segmento) de cada lado.
fn cortar(a: Punto, b: Punto) -> (Punto, Punto) {
    let l = (b.x - a.x).hypot(b.y - a.y);
    let t = if l > 0.0 { (2.0 / l).min(0.25) } else { 0.25 };
    (
        Punto::new(a.x + t * (b.x - a.x), a.y + t * (b.y - a.y)),
        Punto::new(b.x - t * (b.x - a.x), b.y - t * (b.y - a.y)),
    )
}

/// Puntos medios de los segmentos consecutivos. Sobre una escalera de
/// píxeles quedan a ≤ 0.5 px de la recta que aproxima, así Douglas–Peucker
/// la puede colapsar; las esquinas mismas se apartan hasta ~1 px.
fn puntos_medios(p: &[Punto]) -> Vec<Punto> {
    p.windows(2)
        .map(|s| Punto::new((s[0].x + s[1].x) / 2.0, (s[0].y + s[1].y) / 2.0))
        .collect()
}

fn suavizar_abierta(camino: &[Punto]) -> Polilinea {
    // Los extremos (cruces) quedan fijos; el interior pasa a puntos medios.
    let mut medios = vec![camino[0]];
    medios.extend(puntos_medios(camino));
    medios.push(*camino.last().unwrap());
    let mut actual = simplificar(&medios);
    for _ in 0..ITERACIONES {
        if actual.len() < 3 {
            break;
        }
        let mut siguiente = vec![actual[0]];
        for s in actual.windows(2) {
            let (q, r) = cortar(s[0], s[1]);
            siguiente.push(q);
            siguiente.push(r);
        }
        siguiente.push(*actual.last().unwrap());
        actual = siguiente;
    }
    actual
}

fn suavizar_cerrada(esquinas: &[Punto]) -> Polilinea {
    let mut cerrado = esquinas.to_vec();
    cerrado.push(esquinas[0]);
    let anillo = puntos_medios(&cerrado);
    // Simplifica dos mitades cortadas en el punto más lejano al primero.
    let lejano = (1..anillo.len())
        .max_by(|&i, &j| {
            let d = |k: usize| (anillo[k].x - anillo[0].x).hypot(anillo[k].y - anillo[0].y);
            d(i).total_cmp(&d(j))
        })
        .unwrap_or(0);
    let mut actual: Vec<Punto> = if lejano == 0 {
        anillo.to_vec()
    } else {
        let mut a = simplificar(&anillo[..=lejano]);
        let mut resto = anillo[lejano..].to_vec();
        resto.push(anillo[0]);
        let b = simplificar(&resto);
        a.pop();
        a.extend(&b[..b.len() - 1]);
        a
    };
    for _ in 0..ITERACIONES {
        let n = actual.len();
        let mut siguiente = Vec::with_capacity(n * 2);
        for i in 0..n {
            let (q, r) = cortar(actual[i], actual[(i + 1) % n]);
            siguiente.push(q);
            siguiente.push(r);
        }
        actual = siguiente;
    }
    actual.push(actual[0]);
    actual
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grilla(ancho: usize, alto: usize, f: impl Fn(usize, usize) -> u16) -> Etiquetas {
        let mut datos = Vec::new();
        for y in 0..alto {
            for x in 0..ancho {
                datos.push(f(x, y));
            }
        }
        Etiquetas { ancho, alto, datos }
    }

    fn de_par(b: &[Borde], a: u16, c: u16) -> Vec<&Borde> {
        b.iter().filter(|x| x.a == a && x.b == c).collect()
    }

    fn caja(p: &[Punto]) -> (f64, f64, f64, f64) {
        let f = |g: fn(&Punto) -> f64, m: fn(f64, f64) -> f64, i: f64| p.iter().map(g).fold(i, m);
        (
            f(|p| p.x, f64::min, f64::MAX),
            f(|p| p.y, f64::min, f64::MAX),
            f(|p| p.x, f64::max, f64::MIN),
            f(|p| p.y, f64::max, f64::MIN),
        )
    }

    fn cerca(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1.5
    }

    #[test]
    fn cuadrado_en_fondo_da_un_borde_cerrado() {
        let e = grilla(30, 30, |x, y| {
            u16::from((5..25).contains(&x) && (5..25).contains(&y))
        });
        let b = extraer_bordes(&e);
        assert_eq!(b.len(), 1);
        assert_eq!((b[0].a, b[0].b), (0, 1));
        assert_eq!(b[0].puntos.first(), b[0].puntos.last());
        let (x0, y0, x1, y1) = caja(&b[0].puntos);
        assert!(cerca(x0, 5.0) && cerca(y0, 5.0) && cerca(x1, 25.0) && cerca(y1, 25.0));
    }

    #[test]
    fn mitades_comparten_extremos_exactos() {
        let e = grilla(20, 10, |x, _| if x < 10 { 1 } else { 2 });
        let b = extraer_bordes(&e);
        assert_eq!(b.len(), 3);
        let union = de_par(&b, 1, 2);
        assert_eq!(union.len(), 1);
        let u = &union[0].puntos;
        for p in u {
            assert!((p.x - 10.0).abs() < 0.5);
        }
        let extremos_de = |a, c| -> Vec<Punto> {
            de_par(&b, a, c)
                .iter()
                .flat_map(|x| [x.puntos[0], *x.puntos.last().unwrap()])
                .collect()
        };
        for e in [u[0], *u.last().unwrap()] {
            assert!(extremos_de(0, 1).contains(&e));
            assert!(extremos_de(0, 2).contains(&e));
        }
    }

    #[test]
    fn figura_que_toca_el_borde_queda_cerrada() {
        let e = grilla(10, 10, |_, _| 1);
        let b = extraer_bordes(&e);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].puntos.first(), b[0].puntos.last());
        let (x0, y0, x1, y1) = caja(&b[0].puntos);
        assert!(cerca(x0, 0.0) && cerca(y0, 0.0) && cerca(x1, 10.0) && cerca(y1, 10.0));
    }

    #[test]
    fn pieza_rodeada_es_un_lazo_cerrado() {
        let e = grilla(30, 30, |x, y| {
            if (10..20).contains(&x) && (10..20).contains(&y) {
                2
            } else {
                1
            }
        });
        let b = extraer_bordes(&e);
        let l = de_par(&b, 1, 2);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].puntos.first(), l[0].puntos.last());
        assert!(l[0].puntos.len() > 4);
    }

    #[test]
    fn disco_es_suave_y_fiel() {
        let e = grilla(100, 100, |x, y| {
            let (dx, dy) = (x as f64 + 0.5 - 50.0, y as f64 + 0.5 - 50.0);
            u16::from(dx * dx + dy * dy <= 1600.0)
        });
        let b = extraer_bordes(&e);
        assert_eq!(b.len(), 1);
        let p = &b[0].puntos;
        for q in p {
            let r = (q.x - 50.0).hypot(q.y - 50.0);
            assert!((r - 40.0).abs() <= 1.5, "radio {r}");
        }
        for w in p.windows(3) {
            let a1 = (w[1].y - w[0].y).atan2(w[1].x - w[0].x);
            let a2 = (w[2].y - w[1].y).atan2(w[2].x - w[1].x);
            let mut d = (a2 - a1).abs();
            if d > std::f64::consts::PI {
                d = 2.0 * std::f64::consts::PI - d;
            }
            assert!(d.to_degrees() <= 30.0, "ángulo {}", d.to_degrees());
        }
    }

    #[test]
    fn y_comparte_un_cruce() {
        let e = grilla(20, 20, |x, y| {
            if x < 10 {
                1
            } else if y < 10 {
                2
            } else {
                3
            }
        });
        let b = extraer_bordes(&e);
        let cruce = Punto::new(10.0, 10.0);
        for (a, c) in [(1, 2), (1, 3), (2, 3)] {
            let v = de_par(&b, a, c);
            assert_eq!(v.len(), 1, "par {a},{c}");
            let p = &v[0].puntos;
            assert!(p[0] == cruce || *p.last().unwrap() == cruce, "par {a},{c}");
        }
    }

    #[test]
    fn damero_corta_en_el_cruce() {
        let e = grilla(2, 2, |x, y| u16::from((x + y) % 2 == 0));
        let b = extraer_bordes(&e);
        let c = Punto::new(1.0, 1.0);
        assert_eq!(b.len(), 2);
        for x in &b {
            assert_eq!((x.a, x.b), (0, 1));
            assert_eq!(x.puntos[0], c);
            assert_eq!(*x.puntos.last().unwrap(), c);
        }
    }

    #[test]
    fn es_determinista_y_ordenado() {
        let e = grilla(30, 30, |x, y| ((x / 10) + (y / 15) * 3) as u16 % 4);
        let b = extraer_bordes(&e);
        assert_eq!(b, extraer_bordes(&e));
        for par in b.windows(2) {
            let (p, q) = (&par[0], &par[1]);
            let clave = |x: &Borde| (x.a, x.b, x.puntos[0].x, x.puntos[0].y);
            assert!(clave(p) <= clave(q), "{:?} > {:?}", clave(p), clave(q));
        }
    }

    #[test]
    fn union_diagonal_es_recta_y_fiel() {
        // Recta y = 10 + 0.37 x: pendiente que no cae en fase con la grilla.
        let recta = |x: f64| 10.0 + 0.37 * x;
        let e = grilla(80, 60, |x, y| {
            let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
            if !(5..75).contains(&x) || !(5..55).contains(&y) {
                0
            } else if py < recta(px) {
                1
            } else {
                2
            }
        });
        let b = extraer_bordes(&e);
        let union = de_par(&b, 1, 2);
        assert_eq!(union.len(), 1);
        let p = &union[0].puntos;
        let norma = 1.0f64.hypot(0.37);
        for q in p {
            let d = (q.y - recta(q.x)).abs() / norma;
            assert!(d <= 1.5, "desvío {d} en {q:?}");
        }
        for w in p.windows(3) {
            let a1 = (w[1].y - w[0].y).atan2(w[1].x - w[0].x);
            let a2 = (w[2].y - w[1].y).atan2(w[2].x - w[1].x);
            let d = (a2 - a1).abs().to_degrees();
            assert!(d <= 10.0, "ángulo {d}");
        }
    }
}
