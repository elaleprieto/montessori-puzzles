//! Ronda 02: división de la máscara en Piezas. Ver docs/adr/0001.

use crate::mascara::Mascara;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

/// Etiqueta de fondo en una grilla de Etiquetas.
pub const FONDO: u16 = 0;

/// Grilla del mismo tamaño que la Máscara donde cada píxel lleva el número de
/// su Pieza (1..=N) o `FONDO`. Se indexa por fila: `datos[y * ancho + x]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Etiquetas {
    pub ancho: usize,
    pub alto: usize,
    pub datos: Vec<u16>,
}

impl Etiquetas {
    pub fn get(&self, x: usize, y: usize) -> u16 {
        self.datos[y * self.ancho + x]
    }
}

/// Costo de un paso ortogonal en Dijkstra (enteros escalados).
const PASO: i64 = 1000;
/// Costo de un paso diagonal: PASO × √2.
const PASO_DIAG: i64 = 1414;
/// Tope de iteraciones de balance (Lloyd + pesos aditivos).
const ITERACIONES_MAX: usize = 20;
/// Con todas las Piezas a menos de este desvío relativo del área media, el
/// balance se da por terminado.
const TOLERANCIA: f64 = 0.15;
/// Iteraciones seguidas sin mejorar el peor desvío que cortan el balance.
const PACIENCIA: usize = 8;

/// Divide la figura en `piezas` Piezas conexas de área pareja. La misma
/// `semilla` produce siempre la misma Variante.
pub fn particionar(mascara: &Mascara, piezas: usize, semilla: u64) -> Etiquetas {
    assert!(piezas >= 1, "particionar: piezas debe ser >= 1");
    let (ancho, alto) = (mascara.ancho, mascara.alto);
    let n_fig = mascara.datos.iter().filter(|&&b| b).count();
    let mut out = Etiquetas {
        ancho,
        alto,
        datos: vec![FONDO; ancho * alto],
    };
    if n_fig == 0 {
        return out;
    }
    assert!(
        piezas <= n_fig,
        "particionar: {piezas} piezas pedidas pero la figura solo tiene {n_fig} píxeles"
    );
    if piezas == 1 {
        for (d, &m) in out.datos.iter_mut().zip(&mascara.datos) {
            if m {
                *d = 1;
            }
        }
        return out;
    }

    let mut rng = ChaCha8Rng::seed_from_u64(semilla);
    let pixeles: Vec<usize> = (0..ancho * alto).filter(|&i| mascara.datos[i]).collect();
    let mut semillas = semillas_iniciales(&pixeles, ancho, piezas, &mut rng);
    let mut offsets = vec![0i64; piezas];
    let media = n_fig as f64 / piezas as f64;
    let escala = media.sqrt() * PASO as f64 * 0.3;

    // La mejor asignación vista (con las N Piezas presentes) y su peor desvío.
    let mut mejor: Option<(f64, Vec<u16>)> = None;
    let mut sin_mejora = 0;
    for _ in 0..ITERACIONES_MAX {
        let etiquetas = asignar(mascara, &semillas, &offsets);
        let (areas, centroides) = medir_piezas(&etiquetas, &pixeles, ancho, piezas);
        let peor = areas
            .iter()
            .map(|&a| ((a as f64 / media) - 1.0).abs())
            .fold(0.0, f64::max);
        let completa = areas.iter().all(|&a| a > 0);
        if completa && mejor.as_ref().is_none_or(|(p, _)| peor < *p) {
            mejor = Some((peor, etiquetas.clone()));
            sin_mejora = 0;
            if peor <= TOLERANCIA {
                break;
            }
        } else {
            sin_mejora += 1;
            if sin_mejora >= PACIENCIA {
                break;
            }
        }
        mover_semillas_a_centroides(&mut semillas, &etiquetas, &pixeles, ancho, &centroides);
        // Pesos aditivos: Piezas chicas bajan su offset (crecen), grandes lo suben.
        for (o, &a) in offsets.iter_mut().zip(&areas) {
            *o += ((a as f64 - media) / media * escala) as i64;
        }
        let min = *offsets.iter().min().unwrap();
        for o in &mut offsets {
            *o -= min;
        }
    }
    let mut etiquetas = match mejor {
        Some((_, e)) => e,
        None => asignar(mascara, &semillas, &vec![0; piezas]),
    };
    asegurar_conexidad(&mut etiquetas, ancho, alto);
    out.datos = etiquetas;
    out
}

/// Área y centroide de cada Pieza (índice = etiqueta - 1).
fn medir_piezas(
    etiquetas: &[u16],
    pixeles: &[usize],
    ancho: usize,
    piezas: usize,
) -> (Vec<usize>, Vec<Option<(f64, f64)>>) {
    let mut areas = vec![0usize; piezas];
    let mut suma = vec![(0f64, 0f64); piezas];
    for &i in pixeles {
        let k = etiquetas[i] as usize - 1;
        areas[k] += 1;
        suma[k].0 += (i % ancho) as f64;
        suma[k].1 += (i / ancho) as f64;
    }
    let centroides = areas
        .iter()
        .zip(&suma)
        .map(|(&a, &(sx, sy))| (a > 0).then(|| (sx / a as f64, sy / a as f64)))
        .collect();
    (areas, centroides)
}

/// Lloyd, en una sola pasada: cada semilla va al píxel de su Pieza más
/// cercano al centroide de la Pieza. Las Piezas vacías no mueven su semilla.
fn mover_semillas_a_centroides(
    semillas: &mut [usize],
    etiquetas: &[u16],
    pixeles: &[usize],
    ancho: usize,
    centroides: &[Option<(f64, f64)>],
) {
    let mut mas_cercano = vec![(f64::MAX, 0usize); semillas.len()];
    for &i in pixeles {
        let k = etiquetas[i] as usize - 1;
        if let Some((cx, cy)) = centroides[k] {
            let d = ((i % ancho) as f64 - cx).powi(2) + ((i / ancho) as f64 - cy).powi(2);
            if d < mas_cercano[k].0 {
                mas_cercano[k] = (d, i);
            }
        }
    }
    for (s, &(d, i)) in semillas.iter_mut().zip(&mas_cercano) {
        if d < f64::MAX {
            *s = i;
        }
    }
}

fn semillas_iniciales(
    pixeles: &[usize],
    ancho: usize,
    k: usize,
    rng: &mut ChaCha8Rng,
) -> Vec<usize> {
    let mut semillas = vec![pixeles[rng.random_range(0..pixeles.len())]];
    let dist2 = |a: usize, b: usize| {
        let dx = (a % ancho) as f64 - (b % ancho) as f64;
        let dy = (a / ancho) as f64 - (b / ancho) as f64;
        dx * dx + dy * dy
    };
    let mut dist: Vec<f64> = pixeles.iter().map(|&p| dist2(p, semillas[0])).collect();
    while semillas.len() < k {
        let total: f64 = dist.iter().sum();
        let elegido = if total <= 0.0 {
            // Degenerado: cualquier píxel no usado.
            pixeles.iter().position(|p| !semillas.contains(p)).unwrap()
        } else {
            let mut t = rng.random::<f64>() * total;
            let mut e = dist.len() - 1;
            for (j, &d) in dist.iter().enumerate() {
                if d > 0.0 && t < d {
                    e = j;
                    break;
                }
                t -= d;
            }
            if dist[e] == 0.0 {
                e = dist.iter().position(|&d| d > 0.0).unwrap();
            }
            e
        };
        let s = pixeles[elegido];
        semillas.push(s);
        for (j, &p) in pixeles.iter().enumerate() {
            dist[j] = dist[j].min(dist2(p, s));
        }
    }
    semillas
}

/// Dijkstra multi-fuente con vecindad 8 y distancias iniciales `offsets`.
fn asignar(mascara: &Mascara, semillas: &[usize], offsets: &[i64]) -> Vec<u16> {
    let (ancho, alto) = (mascara.ancho, mascara.alto);
    let mut etiquetas = vec![FONDO; ancho * alto];
    let mut dist = vec![i64::MAX; ancho * alto];
    let mut heap = BinaryHeap::new();
    for (k, &s) in semillas.iter().enumerate() {
        if offsets[k] < dist[s] {
            dist[s] = offsets[k];
            etiquetas[s] = k as u16 + 1;
            heap.push(Reverse((offsets[k], s, k as u16 + 1)));
        }
    }
    while let Some(Reverse((d, i, l))) = heap.pop() {
        if d > dist[i] || etiquetas[i] != l {
            continue;
        }
        let (x, y) = ((i % ancho) as isize, (i / ancho) as isize);
        for dy in -1..=1isize {
            for dx in -1..=1isize {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= ancho as isize || ny >= alto as isize {
                    continue;
                }
                let j = ny as usize * ancho + nx as usize;
                if !mascara.datos[j] {
                    continue;
                }
                let nd = d + if dx != 0 && dy != 0 { PASO_DIAG } else { PASO };
                if nd < dist[j] || (nd == dist[j] && l < etiquetas[j]) {
                    dist[j] = nd;
                    etiquetas[j] = l;
                    heap.push(Reverse((nd, j, l)));
                }
            }
        }
    }
    // Píxeles de otras componentes sin semilla: etiqueta de la semilla más cercana.
    for (i, (e, &m)) in etiquetas.iter_mut().zip(&mascara.datos).enumerate() {
        if m && *e == FONDO {
            let mut mas_cercana = (f64::MAX, 1u16);
            for (k, &s) in semillas.iter().enumerate() {
                let dx = (i % ancho) as f64 - (s % ancho) as f64;
                let dy = (i / ancho) as f64 - (s / ancho) as f64;
                let d = dx * dx + dy * dy;
                if d < mas_cercana.0 {
                    mas_cercana = (d, k as u16 + 1);
                }
            }
            *e = mas_cercana.1;
        }
    }
    etiquetas
}

/// Componentes 4-conexas (no fondo) con la misma etiqueta.
fn componentes(etiquetas: &[u16], ancho: usize, alto: usize) -> Vec<(u16, Vec<usize>)> {
    let mut visto = vec![false; ancho * alto];
    let mut res = Vec::new();
    for ini in 0..ancho * alto {
        if etiquetas[ini] == FONDO || visto[ini] {
            continue;
        }
        let l = etiquetas[ini];
        let mut comp = vec![ini];
        visto[ini] = true;
        let mut idx = 0;
        while idx < comp.len() {
            let i = comp[idx];
            idx += 1;
            for j in vecinos4(i, ancho, alto) {
                if !visto[j] && etiquetas[j] == l {
                    visto[j] = true;
                    comp.push(j);
                }
            }
        }
        res.push((l, comp));
    }
    res
}

fn vecinos4(i: usize, ancho: usize, alto: usize) -> impl Iterator<Item = usize> {
    let (x, y) = (i % ancho, i / ancho);
    [
        (x > 0).then(|| i - 1),
        (x + 1 < ancho).then(|| i + 1),
        (y > 0).then(|| i - ancho),
        (y + 1 < alto).then(|| i + ancho),
    ]
    .into_iter()
    .flatten()
}

/// Deja cada etiqueta 4-conexa: toda componente que no sea la más grande de
/// su etiqueta pasa a la etiqueta vecina más frecuente, hasta estabilizar.
fn asegurar_conexidad(etiquetas: &mut [u16], ancho: usize, alto: usize) {
    loop {
        let comps = componentes(etiquetas, ancho, alto);
        let mut mayor = BTreeMap::<u16, usize>::new();
        for (c, (l, comp)) in comps.iter().enumerate() {
            let e = mayor.entry(*l).or_insert(c);
            if comp.len() > comps[*e].1.len() {
                *e = c;
            }
        }
        let mut cambio = false;
        for (c, (l, comp)) in comps.iter().enumerate() {
            if mayor[l] == c {
                continue;
            }
            // Vecino más frecuente (empate: etiqueta menor).
            let mut cuenta = BTreeMap::<u16, usize>::new();
            for &i in comp {
                for j in vecinos4(i, ancho, alto) {
                    if etiquetas[j] != FONDO && etiquetas[j] != *l {
                        *cuenta.entry(etiquetas[j]).or_default() += 1;
                    }
                }
            }
            let mut elegido = None;
            for (&e, &n) in &cuenta {
                if elegido.is_none_or(|(_, m)| n > m) {
                    elegido = Some((e, n));
                }
            }
            if let Some((e, _)) = elegido {
                for &i in comp {
                    etiquetas[i] = e;
                }
                cambio = true;
            }
        }
        if !cambio {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn forma(w: usize, h: usize, f: impl Fn(usize, usize) -> bool) -> Mascara {
        let mut m = Mascara::vacia(w, h);
        for y in 0..h {
            for x in 0..w {
                m.set(x, y, f(x, y));
            }
        }
        m
    }
    fn rect() -> Mascara {
        forma(220, 120, |x, y| {
            (10..210).contains(&x) && (10..110).contains(&y)
        })
    }
    fn disco() -> Mascara {
        forma(120, 120, |x, y| {
            (x as i32 - 60).pow(2) + (y as i32 - 60).pow(2) <= 50 * 50
        })
    }
    fn ele() -> Mascara {
        forma(140, 140, |x, y| {
            (10..130).contains(&x) && (10..130).contains(&y) && (x < 50 || y >= 90)
        })
    }
    fn u() -> Mascara {
        forma(160, 140, |x, y| {
            (10..150).contains(&x) && (10..130).contains(&y) && (!(50..110).contains(&x) || y >= 90)
        })
    }
    fn mancuerna() -> Mascara {
        forma(200, 80, |x, y| {
            let d = |cx: i32| (x as i32 - cx).pow(2) + (y as i32 - 40).pow(2);
            d(35) <= 30 * 30
                || d(165) <= 30 * 30
                || ((60..140).contains(&x) && (36..44).contains(&y))
        })
    }

    /// Flood fill 4-conexo propio del test, independiente de `componentes`.
    fn es_4_conexa(e: &Etiquetas, l: u16) -> bool {
        let total = e.datos.iter().filter(|&&x| x == l).count();
        let Some(ini) = e.datos.iter().position(|&x| x == l) else {
            return true;
        };
        let mut visto = vec![false; e.datos.len()];
        let mut pila = vec![(ini % e.ancho, ini / e.ancho)];
        visto[ini] = true;
        let mut alcanzados = 0;
        while let Some((x, y)) = pila.pop() {
            alcanzados += 1;
            let vecinos = [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ];
            for (nx, ny) in vecinos {
                if nx < e.ancho && ny < e.alto {
                    let j = ny * e.ancho + nx;
                    if !visto[j] && e.datos[j] == l {
                        visto[j] = true;
                        pila.push((nx, ny));
                    }
                }
            }
        }
        alcanzados == total
    }

    /// Fondo en 0, figura en 1..=n y aparecen exactamente n etiquetas.
    fn verificar_etiquetas(m: &Mascara, e: &Etiquetas, n: usize) {
        let mut vistas = BTreeSet::new();
        for (i, &l) in e.datos.iter().enumerate() {
            if m.datos[i] {
                assert!((1..=n as u16).contains(&l));
                vistas.insert(l);
            } else {
                assert_eq!(l, FONDO);
            }
        }
        assert_eq!(vistas.len(), n);
    }

    fn verificar_basico(m: &Mascara, e: &Etiquetas, n: usize) {
        verificar_etiquetas(m, e, n);
        for l in 1..=n as u16 {
            assert!(es_4_conexa(e, l), "etiqueta {l} no es 4-conexa");
        }
    }

    fn areas_parejas(m: &Mascara, n: usize) {
        let e = particionar(m, n, 7);
        verificar_basico(m, &e, n);
        let prom = m.datos.iter().filter(|&&b| b).count() as f64 / n as f64;
        for l in 1..=n as u16 {
            let a = e.datos.iter().filter(|&&x| x == l).count() as f64;
            assert!(
                a >= 0.6 * prom && a <= 1.5 * prom,
                "n={n} etiqueta {l}: área {a} vs promedio {prom}"
            );
        }
    }

    #[test]
    fn etiquetas_validas_y_conexas_en_todas_las_figuras() {
        for m in [rect(), disco(), ele(), u(), mancuerna()] {
            for n in [2, 5, 8, 12] {
                for semilla in 0..2 {
                    verificar_basico(&m, &particionar(&m, n, semilla), n);
                }
            }
        }
    }

    #[test]
    fn areas_parejas_rectangulo() {
        for n in [2, 5, 8] {
            areas_parejas(&rect(), n);
        }
    }
    #[test]
    fn areas_parejas_ele() {
        for n in [2, 5, 8] {
            areas_parejas(&ele(), n);
        }
    }
    #[test]
    fn areas_parejas_u() {
        for n in [2, 5, 8] {
            areas_parejas(&u(), n);
        }
    }

    #[test]
    fn mascara_con_varias_componentes_etiqueta_todos_sus_pixeles() {
        // Tres islas y dos Piezas: al menos una isla queda sin semilla propia.
        let m = forma(90, 30, |x, y| {
            (5..12).contains(&y)
                && ((5..20).contains(&x) || (35..50).contains(&x) || (65..80).contains(&x))
        });
        for semilla in 0..3 {
            verificar_etiquetas(&m, &particionar(&m, 2, semilla), 2);
        }
    }

    #[test]
    fn isla_pasa_a_la_etiqueta_vecina_mas_frecuente() {
        // La isla de 1 en (1,1) toca tres veces el 2 y una vez el 3.
        #[rustfmt::skip]
        let mut etiquetas = vec![
            2, 2, 2, 3, 3,
            2, 1, 3, 3, 3,
            2, 2, 3, 3, 3,
            1, 1, 1, 1, 1,
        ];
        asegurar_conexidad(&mut etiquetas, 5, 4);
        assert_eq!(etiquetas[5 + 1], 2);
        assert_eq!(etiquetas.iter().filter(|&&l| l == 1).count(), 5);
        assert_eq!(etiquetas.iter().filter(|&&l| l == 3).count(), 8);
    }

    #[test]
    fn determinismo_por_semilla() {
        let m = u();
        assert_eq!(particionar(&m, 5, 3), particionar(&m, 5, 3));
        let base = particionar(&m, 5, 0);
        assert!((1..5).any(|s| particionar(&m, 5, s) != base));
    }

    #[test]
    fn una_pieza_cubre_toda_la_figura() {
        let m = ele();
        let e = particionar(&m, 1, 0);
        verificar_basico(&m, &e, 1);
    }

    #[test]
    fn mascara_vacia_es_todo_fondo() {
        let e = particionar(&Mascara::vacia(10, 10), 3, 0);
        assert!(e.datos.iter().all(|&l| l == FONDO));
    }

    #[test]
    #[should_panic]
    fn cero_piezas_entra_en_panico() {
        particionar(&rect(), 0, 0);
    }

    #[test]
    #[should_panic(expected = "píxeles")]
    fn mas_piezas_que_pixeles_entra_en_panico() {
        let mut m = Mascara::vacia(5, 5);
        m.set(1, 1, true);
        m.set(2, 1, true);
        particionar(&m, 3, 0);
    }
}
