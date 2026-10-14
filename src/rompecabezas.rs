//! Ronda 06: el pipeline completo, de la Ilustración al Rompecabezas.

use crate::Parametros;
use crate::bordes::extraer_bordes;
use crate::geometria::{Punto, Rect, Trazo, largo};
use crate::mascara::Mascara;
use crate::mascara::{Formato, Ilustracion, opciones_svg};
use crate::particion::{Etiquetas, FONDO, particionar};
use crate::pestana::{ParametrosPestana, con_pestana};
use anyhow::{Result, bail};
use resvg::usvg;

/// Lado mayor de la grilla de trabajo, en píxeles.
const LADO_MAX: usize = 800;
/// Variantes que se prueban antes de rendirse.
const INTENTOS: u64 = 20;
/// Una Unión más corta que esto (mm) no lleva Pestaña.
const UNION_MIN_MM: f64 = 15.0;
/// Tamaño mínimo de una Pieza respecto del promedio.
const FRACCION_MIN: f64 = 0.5;

/// Resultado de elegir una Variante entre varias.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eleccion {
    pub indice: usize,
    pub cumple: bool,
}

/// Elige la Variante a usar dado el área de cada Pieza de cada candidata.
pub fn elegir_variante(candidatas: &[Vec<usize>]) -> Eleccion {
    let mut mejor = (0, f64::NEG_INFINITY);
    for (indice, areas) in candidatas.iter().enumerate() {
        let r = razon_minima(areas);
        if r >= FRACCION_MIN {
            return Eleccion {
                indice,
                cumple: true,
            };
        }
        if r > mejor.1 {
            mejor = (indice, r);
        }
    }
    Eleccion {
        indice: mejor.0,
        cumple: false,
    }
}

/// Área mínima dividida por el promedio (0 si no hay Piezas).
fn razon_minima(areas: &[usize]) -> f64 {
    if areas.is_empty() {
        return 0.0;
    }
    let media = areas.iter().sum::<usize>() as f64 / areas.len() as f64;
    if media == 0.0 {
        return 0.0;
    }
    *areas.iter().min().unwrap() as f64 / media
}

/// ¿La Ilustración (SVG) usa el rojo de la Capa de corte, #FF0000, en un
/// trazo o relleno? Se mira el árbol ya normalizado por usvg.
pub fn usa_rojo_de_corte(ilustracion: &Ilustracion) -> bool {
    if ilustracion.formato != Formato::Svg {
        return false;
    }
    let Ok(tree) = usvg::Tree::from_data(&ilustracion.bytes, opciones_svg()) else {
        return false;
    };
    grupo_usa_rojo(tree.root())
}

fn es_rojo(pintura: &usvg::Paint) -> bool {
    matches!(pintura, usvg::Paint::Color(c) if (c.red, c.green, c.blue) == (255, 0, 0))
}

fn grupo_usa_rojo(grupo: &usvg::Group) -> bool {
    grupo.children().iter().any(|nodo| match nodo {
        usvg::Node::Group(g) => grupo_usa_rojo(g),
        usvg::Node::Path(p) => {
            p.fill().is_some_and(|f| es_rojo(f.paint()))
                || p.stroke().is_some_and(|s| es_rojo(s.paint()))
        }
        _ => false,
    })
}

/// El resultado de `generar`, ya en milímetros.
#[derive(Debug, Clone)]
pub struct Rompecabezas {
    pub lienzo_mm: (f64, f64),
    pub ilustracion_en: Rect,
    pub silueta: Vec<Trazo>,
    pub uniones: Vec<Trazo>,
    /// Etiquetas de la grilla de trabajo (para colorear Piezas en la vista previa).
    pub etiquetas: Etiquetas,
    pub mm_por_px: f64,
    pub semilla_usada: u64,
    pub aviso: Option<String>,
}

/// Corre el pipeline completo.
pub fn generar(ilustracion: &Ilustracion, p: &Parametros) -> Result<Rompecabezas> {
    if !(2..=12).contains(&p.piezas) {
        bail!("La cantidad de piezas debe estar entre 2 y 12");
    }
    if !(p.ancho_mm > 0.0 && p.ancho_mm.is_finite()) {
        bail!("El ancho debe ser mayor que cero");
    }
    let (w_px, h_px) = tamano_grilla(ilustracion);
    let mm_por_px = p.ancho_mm / w_px as f64;
    let margen_px = p.margen_mm.max(0.0) / mm_por_px;
    let relleno_px = margen_px.ceil() as usize + 2;
    let mascara = Mascara::desde_ilustracion(ilustracion, p.umbral, LADO_MAX, relleno_px)?
        .figura_principal()
        .dilatar(margen_px);
    if !mascara.datos.iter().any(|&b| b) {
        bail!("No se encontró ninguna figura en la Ilustración; probá ajustar el Umbral");
    }

    let (etiquetas, semilla_usada, cumple) = buscar_variante(p.semilla, p.piezas, |semilla| {
        particionar(&mascara, p.piezas, semilla)
    });

    let mut silueta = Vec::new();
    let mut uniones = Vec::new();
    let pestana = ParametrosPestana {
        tamano: p.tamano_pestana,
        jitter: p.jitter,
    };
    for (i, borde) in extraer_bordes(&etiquetas).iter().enumerate() {
        let mm: Vec<Punto> = borde
            .puntos
            .iter()
            .map(|q| Punto::new(q.x * mm_por_px, q.y * mm_por_px))
            .collect();
        if borde.a == FONDO {
            silueta.push(Trazo::desde_polilinea(&mm));
        } else {
            let semilla = semilla_borde(semilla_usada, borde.a, borde.b, i);
            uniones.push(trazo_de_union(&mm, pestana, semilla));
        }
    }

    let aviso = componer_aviso(cumple, semilla_usada, usa_rojo_de_corte(ilustracion));
    let relleno_mm = relleno_px as f64 * mm_por_px;
    Ok(Rompecabezas {
        lienzo_mm: (
            mascara.ancho as f64 * mm_por_px,
            mascara.alto as f64 * mm_por_px,
        ),
        ilustracion_en: Rect {
            x: relleno_mm,
            y: relleno_mm,
            ancho: p.ancho_mm,
            alto: h_px as f64 * mm_por_px,
        },
        silueta,
        uniones,
        etiquetas,
        mm_por_px,
        semilla_usada,
        aviso,
    })
}

/// Prueba hasta `INTENTOS` Variantes (semilla, semilla+1, ...) y devuelve la
/// primera con Piezas parejas; si ninguna cumple, la de mejor razón.
/// Devuelve (Etiquetas, semilla usada, si cumple).
fn buscar_variante(
    semilla: u64,
    piezas: usize,
    mut particionar: impl FnMut(u64) -> Etiquetas,
) -> (Etiquetas, u64, bool) {
    let mut candidatas: Vec<Etiquetas> = Vec::new();
    let mut areas: Vec<Vec<usize>> = Vec::new();
    for intento in 0..INTENTOS {
        let et = particionar(semilla.wrapping_add(intento));
        areas.push(areas_por_pieza(&et, piezas));
        candidatas.push(et);
        if elegir_variante(&areas).cumple {
            break;
        }
    }
    let e = elegir_variante(&areas);
    (
        candidatas.swap_remove(e.indice),
        semilla.wrapping_add(e.indice as u64),
        e.cumple,
    )
}

/// Los avisos de `generar`, uno por línea.
fn componer_aviso(variante_cumple: bool, semilla_usada: u64, rojo: bool) -> Option<String> {
    let mut avisos = Vec::new();
    if !variante_cumple {
        avisos.push(format!(
            "Ninguna Variante cumplió el tamaño mínimo de Pieza; se usa la mejor (semilla {semilla_usada})"
        ));
    }
    if rojo {
        avisos.push(
            "La Ilustración usa el mismo rojo que la Capa de corte (#FF0000); en LightBurn esas líneas se cortarían"
                .to_string(),
        );
    }
    if avisos.is_empty() {
        None
    } else {
        Some(avisos.join("\n"))
    }
}

/// Una Unión de `UNION_MIN_MM` o más lleva Pestaña; una más corta queda recta.
fn trazo_de_union(mm: &[Punto], pestana: ParametrosPestana, semilla: u64) -> Trazo {
    if largo(mm) >= UNION_MIN_MM {
        con_pestana(mm, pestana, semilla)
    } else {
        Trazo::desde_polilinea(mm)
    }
}

/// Tamaño en píxeles de la Ilustración rasterizada con su lado mayor en `LADO_MAX`.
fn tamano_grilla(il: &Ilustracion) -> (usize, usize) {
    let (w, h) = (il.ancho_px as f64, il.alto_px as f64);
    let escala = LADO_MAX as f64 / w.max(h);
    (
        ((w * escala).round() as usize).max(1),
        ((h * escala).round() as usize).max(1),
    )
}

fn areas_por_pieza(et: &Etiquetas, piezas: usize) -> Vec<usize> {
    let mut a = vec![0; piezas];
    for &e in &et.datos {
        if e != FONDO && (e as usize) <= piezas {
            a[e as usize - 1] += 1;
        }
    }
    a
}

/// Semilla determinística de un Borde (mezclador splitmix64).
fn semilla_borde(semilla: u64, a: u16, b: u16, indice: usize) -> u64 {
    let mut z = semilla
        .wrapping_add(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(((a as u64) << 48) ^ ((b as u64) << 32) ^ indice as u64);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// El SVG para el láser: Grabado = Ilustración, Corte = Silueta + Uniones.
pub fn exportar_svg(r: &Rompecabezas, ilustracion: &Ilustracion) -> String {
    crate::svg::exportar(&crate::svg::Dibujo {
        ancho_mm: r.lienzo_mm.0,
        alto_mm: r.lienzo_mm.1,
        cortes: r.silueta.iter().chain(&r.uniones).cloned().collect(),
        ilustracion,
        ilustracion_en: r.ilustracion_en,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svg(cuerpo: &str) -> Ilustracion {
        let s = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">{cuerpo}</svg>"#
        );
        Ilustracion::desde_bytes(s.into_bytes(), Formato::Svg).unwrap()
    }

    fn etiquetas_con_areas(areas: &[usize]) -> Etiquetas {
        let mut datos = Vec::new();
        for (i, &a) in areas.iter().enumerate() {
            datos.extend(std::iter::repeat_n(i as u16 + 1, a));
        }
        Etiquetas {
            ancho: datos.len(),
            alto: 1,
            datos,
        }
    }

    #[test]
    fn busca_variante_reporta_la_semilla_de_la_elegida() {
        // Las semillas 100 y 101 fallan; la 102 cumple.
        let mut llamadas = 0;
        let (et, semilla, cumple) = buscar_variante(100, 2, |s| {
            llamadas += 1;
            etiquetas_con_areas(if s == 102 { &[50, 50] } else { &[5, 95] })
        });
        assert_eq!((semilla, cumple, llamadas), (102, true, 3));
        assert_eq!(et.datos.iter().filter(|&&e| e == 1).count(), 50);
    }

    #[test]
    fn sin_variante_valida_usa_la_mejor_tras_veinte_intentos() {
        let mut llamadas = 0;
        let (_, semilla, cumple) = buscar_variante(10, 2, |s| {
            llamadas += 1;
            // La semilla 14 es la menos mala.
            etiquetas_con_areas(if s == 14 { &[20, 80] } else { &[5, 95] })
        });
        assert_eq!((semilla, cumple, llamadas), (14, false, 20));
    }

    #[test]
    fn aviso_combina_pieza_chica_y_rojo() {
        assert_eq!(componer_aviso(true, 3, false), None);
        let chica = componer_aviso(false, 7, false).unwrap();
        assert!(chica.contains("semilla 7") && !chica.contains("#FF0000"));
        let rojo = componer_aviso(true, 7, true).unwrap();
        assert!(rojo.contains("#FF0000") && !rojo.contains("Variante"));
        let ambos = componer_aviso(false, 7, true).unwrap();
        assert!(ambos.contains("semilla 7") && ambos.contains("#FF0000"));
    }

    #[test]
    fn union_de_15_mm_lleva_pestana_y_las_mas_cortas_no() {
        let pest = ParametrosPestana {
            tamano: 0.2,
            jitter: 0.3,
        };
        let recta = |l: f64| [Punto::new(0.0, 0.0), Punto::new(l, 0.0)];
        let cubicas = |t: &Trazo| {
            t.segmentos
                .iter()
                .any(|s| matches!(s, crate::geometria::Segmento::Cubica(..)))
        };
        assert!(cubicas(&trazo_de_union(&recta(15.0), pest, 1)));
        assert!(!cubicas(&trazo_de_union(&recta(14.9), pest, 1)));
        assert!(!cubicas(&trazo_de_union(&recta(8.0), pest, 1)));
    }

    #[test]
    fn detecta_el_rojo_en_cualquier_forma() {
        for r in ["red", "#f00", "#FF0000", "rgb(255,0,0)"] {
            let trazo = svg(&format!(
                r#"<path d="M0 0 L50 50" stroke="{r}" fill="none"/>"#
            ));
            assert!(usa_rojo_de_corte(&trazo), "trazo {r}");
            let relleno = svg(&format!(r#"<rect width="50" height="50" fill="{r}"/>"#));
            assert!(usa_rojo_de_corte(&relleno), "relleno {r}");
        }
    }

    #[test]
    fn otros_colores_no_avisan() {
        let azul = svg(r##"<rect width="50" height="50" fill="#0000ff" stroke="#fe0000"/>"##);
        assert!(!usa_rojo_de_corte(&azul));
        let anidado = svg(r##"<g><g><rect width="5" height="5" fill="#00ff00"/></g></g>"##);
        assert!(!usa_rojo_de_corte(&anidado));
    }

    #[test]
    fn detecta_rojo_dentro_de_grupos() {
        let il = svg(r#"<g opacity="0.9"><g><circle cx="5" cy="5" r="3" fill="red"/></g></g>"#);
        assert!(usa_rojo_de_corte(&il));
    }

    #[test]
    fn un_raster_nunca_avisa() {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
        let mut b = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut b), image::ImageFormat::Png)
            .unwrap();
        let il = Ilustracion::desde_bytes(b, Formato::Png).unwrap();
        assert!(!usa_rojo_de_corte(&il));
    }

    #[test]
    fn elige_la_primera_que_cumple() {
        let c = vec![vec![10, 100], vec![40, 60], vec![50, 50]];
        assert_eq!(
            elegir_variante(&c),
            Eleccion {
                indice: 1,
                cumple: true
            }
        );
    }

    #[test]
    fn sin_ninguna_que_cumpla_elige_la_de_mayor_razon() {
        // razones: 10/55=.18, 20/55=.36, 15/55=.27 -> la 1
        let c = vec![vec![10, 100], vec![20, 90], vec![15, 95]];
        assert_eq!(
            elegir_variante(&c),
            Eleccion {
                indice: 1,
                cumple: false
            }
        );
    }

    #[test]
    fn el_limite_de_la_mitad_del_promedio_cumple() {
        // promedio 60, mínimo 30 = exactamente 50%
        let c = vec![vec![30, 90]];
        assert!(elegir_variante(&c).cumple);
        let c = vec![vec![29, 91]];
        assert!(!elegir_variante(&c).cumple);
    }
}
