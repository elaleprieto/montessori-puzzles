//! Ronda 06: el pipeline completo, de la Ilustración al Rompecabezas.

use crate::Parametros;
use crate::bordes::extraer_bordes;
use crate::geometria::{Punto, Rect, Trazo, largo, rectangulo_redondeado};
use crate::mascara::Mascara;
use crate::mascara::{Formato, Ilustracion, opciones_svg, rasterizar};
use crate::particion::{self, Etiquetas, FONDO, particionar};
use crate::pestana::{ParametrosPestana, con_pestana};
use anyhow::{Result, bail};
use resvg::usvg;

/// Lado mayor de la grilla de trabajo, en píxeles.
const LADO_MAX: usize = 800;
/// Variantes que se prueban antes de rendirse.
const INTENTOS: u64 = 20;
/// Una Unión más corta que esto (mm) no lleva Pestaña.
const UNION_MIN_MM: f64 = 15.0;
/// Máximo de filas y de columnas de una Cuadrícula.
pub const MAX_LADO_CUADRICULA: usize = 8;
/// Píxeles de fondo alrededor de la figura (además del Margen).
const RELLENO_PX: usize = 2;
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
    let cuadricula = p.silueta_rectangular && p.cuadricula;
    if cuadricula {
        if !(1..=MAX_LADO_CUADRICULA).contains(&p.filas)
            || !(1..=MAX_LADO_CUADRICULA).contains(&p.columnas)
        {
            bail!("Las filas y las columnas deben estar entre 1 y {MAX_LADO_CUADRICULA}");
        }
        if p.filas * p.columnas < 2 {
            bail!("La Cuadrícula debe tener al menos 2 Piezas");
        }
    } else if !(2..=12).contains(&p.piezas) {
        bail!("La cantidad de piezas debe estar entre 2 y 12");
    }
    if !(p.ancho_mm > 0.0 && p.ancho_mm.is_finite()) {
        bail!("El ancho debe ser mayor que cero");
    }
    let (w_px, h_px) = tamano_grilla(ilustracion);
    let mm_por_px = p.ancho_mm / w_px as f64;
    let rectangular = radio_rectangular(p);
    let (mascara, relleno_px) = match rectangular {
        // Ni Umbral ni Margen: la figura es la Ilustración entera.
        Some(radio_mm) => (
            Mascara::rectangulo(w_px, h_px, RELLENO_PX, radio_mm / mm_por_px),
            RELLENO_PX,
        ),
        None => {
            let margen_px = p.margen_mm.max(0.0) / mm_por_px;
            let relleno_px = margen_px.ceil() as usize + RELLENO_PX;
            let m = Mascara::desde_ilustracion(ilustracion, p.umbral, LADO_MAX, relleno_px)?
                .figura_principal()
                .dilatar(margen_px);
            (m, relleno_px)
        }
    };
    let relleno_mm = relleno_px as f64 * mm_por_px;
    let ilustracion_en = Rect {
        x: relleno_mm,
        y: relleno_mm,
        ancho: p.ancho_mm,
        alto: h_px as f64 * mm_por_px,
    };
    if !mascara.datos.iter().any(|&b| b) {
        bail!("No se encontró ninguna figura en la Ilustración; probá ajustar el Umbral");
    }

    let (etiquetas, semilla_usada, pieza_chica) = if cuadricula {
        // Una sola partición posible: la semilla solo decide las Pestañas.
        let et = particion::cuadricula(&mascara, p.filas, p.columnas);
        let areas = areas_por_pieza(&et, p.filas * p.columnas);
        let chica = (razon_minima(&areas) < FRACCION_MIN).then(|| {
            "Hay Piezas de menos de la mitad del promedio; probá con un radio de esquinas menor"
                .to_string()
        });
        (et, p.semilla, chica)
    } else {
        let (et, semilla_usada, cumple) = buscar_variante(p.semilla, p.piezas, |semilla| {
            particionar(&mascara, p.piezas, semilla)
        });
        let chica = (!cumple).then(|| {
            format!(
                "Ninguna Variante cumplió el tamaño mínimo de Pieza; se usa la mejor (semilla {semilla_usada})"
            )
        });
        (et, semilla_usada, chica)
    };

    // La Silueta rectangular es exacta, no la que sale de los píxeles.
    let mut silueta: Vec<Trazo> = rectangular
        .iter()
        .map(|&radio_mm| rectangulo_redondeado(ilustracion_en, radio_mm))
        .collect();
    let mut uniones = Vec::new();
    let pestana = ParametrosPestana {
        tamano: p.tamano_pestana,
        jitter: p.jitter,
    };
    for (i, borde) in extraer_bordes(&etiquetas).iter().enumerate() {
        let mut mm: Vec<Punto> = borde
            .puntos
            .iter()
            .map(|q| Punto::new(q.x * mm_por_px, q.y * mm_por_px))
            .collect();
        if borde.a == FONDO {
            if rectangular.is_none() {
                silueta.push(Trazo::desde_polilinea(&mm));
            }
        } else {
            if let Some(radio_mm) = rectangular {
                let n = mm.len() - 1;
                for i in [0, n] {
                    mm[i] = al_arco(mm[i], ilustracion_en, radio_mm, 2.0 * mm_por_px);
                }
            }
            let semilla = semilla_borde(semilla_usada, borde.a, borde.b, i);
            uniones.push(trazo_de_union(&mm, pestana, semilla));
        }
    }

    let aviso = componer_aviso(pieza_chica, usa_rojo_de_corte(ilustracion));
    Ok(Rompecabezas {
        lienzo_mm: (
            mascara.ancho as f64 * mm_por_px,
            mascara.alto as f64 * mm_por_px,
        ),
        ilustracion_en,
        silueta,
        uniones,
        etiquetas,
        mm_por_px,
        semilla_usada,
        aviso,
    })
}

/// Con Silueta rectangular, el radio de sus esquinas en mm (0 si son rectas).
fn radio_rectangular(p: &Parametros) -> Option<f64> {
    p.silueta_rectangular.then(|| {
        if p.esquinas_redondeadas {
            p.radio_esquinas_mm.max(0.0)
        } else {
            0.0
        }
    })
}

/// Lleva al arco de una esquina redondeada de `r` un extremo de Unión que
/// quedó a menos de `tolerancia` de él: en la grilla, el arco es una escalera
/// y la Unión podría no llegar a la Silueta, dejando las Piezas pegadas.
/// Como depende solo del punto, las Uniones que comparten un extremo lo
/// siguen compartiendo.
fn al_arco(p: Punto, r: Rect, radio: f64, tolerancia: f64) -> Punto {
    let radio = radio.clamp(0.0, r.ancho.min(r.alto) / 2.0);
    let cx = p.x.clamp(r.x + radio, r.x + r.ancho - radio);
    let cy = p.y.clamp(r.y + radio, r.y + r.alto - radio);
    let d = (p.x - cx).hypot(p.y - cy);
    let en_esquina = radio > 0.0 && cx != p.x && cy != p.y;
    if !en_esquina || (d - radio).abs() > tolerancia {
        return p;
    }
    Punto::new(cx + (p.x - cx) * radio / d, cy + (p.y - cy) * radio / d)
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
fn componer_aviso(pieza_chica: Option<String>, rojo: bool) -> Option<String> {
    let mut avisos: Vec<String> = pieza_chica.into_iter().collect();
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

/// El SVG para el láser: Grabado = Ilustración sin el fondo, Corte = Silueta + Uniones.
pub fn exportar_svg(r: &Rompecabezas, ilustracion: &Ilustracion) -> String {
    let sin_fondo = grabado_sin_fondo(r, ilustracion);
    debug_assert!(
        sin_fondo.is_ok(),
        "no se pudo quitar el fondo: {sin_fondo:?}"
    );
    let (ilustracion, en) = match &sin_fondo {
        Ok((il, en)) => (il, *en),
        Err(_) => (ilustracion, r.ilustracion_en),
    };
    // La página se ajusta a la caja de la Silueta: todo se corre al origen.
    let pagina = caja_de(&r.silueta).unwrap_or(Rect {
        x: 0.0,
        y: 0.0,
        ancho: r.lienzo_mm.0,
        alto: r.lienzo_mm.1,
    });
    crate::svg::exportar(&crate::svg::Dibujo {
        ancho_mm: pagina.ancho,
        alto_mm: pagina.alto,
        cortes: r
            .silueta
            .iter()
            .chain(&r.uniones)
            .map(|t| t.trasladado(-pagina.x, -pagina.y))
            .collect(),
        ilustracion,
        ilustracion_en: Rect {
            x: en.x - pagina.x,
            y: en.y - pagina.y,
            ..en
        },
    })
}

/// Caja de un conjunto de Trazos (con sus puntos de control); `None` si no hay.
fn caja_de(trazos: &[Trazo]) -> Option<Rect> {
    let mut puntos = trazos.iter().flat_map(Trazo::puntos);
    let p = puntos.next()?;
    let (x0, y0, x1, y1) = puntos.fold((p.x, p.y, p.x, p.y), |c, p| {
        (c.0.min(p.x), c.1.min(p.y), c.2.max(p.x), c.3.max(p.y))
    });
    Some(Rect {
        x: x0,
        y: y0,
        ancho: x1 - x0,
        alto: y1 - y0,
    })
}

/// La Ilustración tal como va a la Capa de grabado y dónde va (mm). Si es
/// raster: en PNG a su resolución original, recortada a la caja de la Silueta
/// y con lo que queda fuera de ella transparente, para que el láser no grabe
/// el fondo (ni lo que se cae al cortar). Un SVG va tal cual.
pub fn grabado_sin_fondo(
    r: &Rompecabezas,
    ilustracion: &Ilustracion,
) -> Result<(Ilustracion, Rect)> {
    if ilustracion.formato == Formato::Svg {
        return Ok((ilustracion.clone(), r.ilustracion_en));
    }
    let lado = ilustracion.ancho_px.max(ilustracion.alto_px) as usize;
    let mut img = rasterizar(ilustracion, lado)?;
    let (w, h) = img.dimensions();
    let il = r.ilustracion_en;
    let et = &r.etiquetas;
    // Caja (x0, y0, x1, y1) de los píxeles que quedan dentro, en px.
    let mut caja = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, px) in img.enumerate_pixels_mut() {
        // Centro del píxel, en mm y luego en la grilla de trabajo.
        let mx = il.x + (x as f64 + 0.5) * il.ancho / w as f64;
        let my = il.y + (y as f64 + 0.5) * il.alto / h as f64;
        let (gx, gy) = (mx / r.mm_por_px, my / r.mm_por_px);
        let dentro = gx >= 0.0
            && gy >= 0.0
            && (gx as usize) < et.ancho
            && (gy as usize) < et.alto
            && et.datos[gy as usize * et.ancho + gx as usize] != FONDO;
        if dentro {
            caja = (caja.0.min(x), caja.1.min(y), caja.2.max(x), caja.3.max(y));
        } else {
            px[3] = 0;
        }
    }
    if caja.0 > caja.2 {
        bail!("La Silueta no cubre ningún píxel de la Ilustración");
    }
    let (cw, ch) = (caja.2 - caja.0 + 1, caja.3 - caja.1 + 1);
    let recorte = image::imageops::crop_imm(&img, caja.0, caja.1, cw, ch).to_image();
    let mut png = Vec::new();
    recorte.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)?;
    let (mm_x, mm_y) = (il.ancho / w as f64, il.alto / h as f64);
    let en = Rect {
        x: il.x + caja.0 as f64 * mm_x,
        y: il.y + caja.1 as f64 * mm_y,
        ancho: cw as f64 * mm_x,
        alto: ch as f64 * mm_y,
    };
    Ok((Ilustracion::desde_bytes(png, Formato::Png)?, en))
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
        let chica = || Some("Pieza chica".to_string());
        assert_eq!(componer_aviso(None, false), None);
        assert_eq!(componer_aviso(chica(), false).unwrap(), "Pieza chica");
        let rojo = componer_aviso(None, true).unwrap();
        assert!(rojo.contains("#FF0000") && !rojo.contains("Pieza chica"));
        let ambos = componer_aviso(chica(), true).unwrap();
        assert!(ambos.starts_with("Pieza chica\n") && ambos.contains("#FF0000"));
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

    /// Distancia de `p` (dentro de `r`) al contorno de `r` con esquinas de `radio`.
    fn distancia_al_contorno(p: Punto, r: Rect, radio: f64) -> f64 {
        let radio = radio.min(r.ancho.min(r.alto) / 2.0);
        let cx = p.x.clamp(r.x + radio, r.x + r.ancho - radio);
        let cy = p.y.clamp(r.y + radio, r.y + r.alto - radio);
        if radio > 0.0 && cx != p.x && cy != p.y {
            return ((p.x - cx).hypot(p.y - cy) - radio).abs();
        }
        let lados = [
            p.x - r.x,
            r.x + r.ancho - p.x,
            p.y - r.y,
            r.y + r.alto - p.y,
        ];
        lados.into_iter().fold(f64::MAX, f64::min)
    }

    /// Una Ilustración transparente: sin Silueta rectangular no tiene figura.
    fn png_transparente(ancho: u32, alto: u32) -> Ilustracion {
        let img = image::RgbaImage::new(ancho, alto);
        let mut b = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut b), image::ImageFormat::Png)
            .unwrap();
        Ilustracion::desde_bytes(b, Formato::Png).unwrap()
    }

    #[test]
    fn silueta_rectangular_ignora_la_figura_y_sigue_el_borde_de_la_ilustracion() {
        let il = png_transparente(300, 200);
        let p = Parametros {
            margen_mm: 5.0,
            ..Parametros::default()
        };
        assert!(generar(&il, &p).is_err());
        let r = generar(
            &il,
            &Parametros {
                silueta_rectangular: true,
                ..p
            },
        )
        .unwrap();
        assert_eq!(
            r.silueta,
            vec![rectangulo_redondeado(r.ilustracion_en, 0.0)]
        );
        assert!((r.ilustracion_en.ancho - p.ancho_mm).abs() < 1e-9);
        let caja = caja_de(&r.silueta).unwrap();
        assert!((caja.alto - p.ancho_mm * 200.0 / 300.0).abs() < r.mm_por_px);
        assert_eq!(areas_por_pieza(&r.etiquetas, p.piezas).len(), p.piezas);
    }

    #[test]
    fn las_uniones_llegan_justo_a_la_silueta_rectangular() {
        let il = png_transparente(300, 200);
        for radio in [0.0, 12.0, 100.0] {
            for semilla in 0..2 {
                let p = Parametros {
                    piezas: 6,
                    semilla,
                    silueta_rectangular: true,
                    esquinas_redondeadas: true,
                    radio_esquinas_mm: radio,
                    ..Parametros::default()
                };
                let r = generar(&il, &p).unwrap();
                let franja = 2.5 * r.mm_por_px;
                let mut en_silueta = 0;
                for u in &r.uniones {
                    let fin = u.puntos().last().unwrap();
                    for q in [u.inicio, fin] {
                        let d = distancia_al_contorno(q, r.ilustracion_en, radio);
                        assert!(
                            d < 1e-6 || d > franja,
                            "radio {radio}, semilla {semilla}: extremo {q:?} a {d} mm"
                        );
                        en_silueta += usize::from(d < 1e-6);
                    }
                }
                assert!(en_silueta > 0);
            }
        }
    }

    fn en_cuadricula(filas: usize, columnas: usize) -> Parametros {
        Parametros {
            silueta_rectangular: true,
            cuadricula: true,
            filas,
            columnas,
            ..Parametros::default()
        }
    }

    #[test]
    fn cuadricula_da_piezas_iguales_con_una_pestana_por_union() {
        let il = png_transparente(400, 300);
        let p = en_cuadricula(3, 4);
        let r = generar(&il, &p).unwrap();
        let areas = areas_por_pieza(&r.etiquetas, 12);
        assert!(razon_minima(&areas) > 0.95, "{areas:?}");
        // 3 filas de 3 Uniones verticales y 2 filas de 4 horizontales.
        assert_eq!(r.uniones.len(), 3 * 3 + 2 * 4);
        for u in &r.uniones {
            let cubicas = u
                .segmentos
                .iter()
                .any(|s| matches!(s, crate::geometria::Segmento::Cubica(..)));
            assert!(cubicas, "Unión sin Pestaña");
        }
        assert_eq!((r.semilla_usada, r.aviso.clone()), (p.semilla, None));
        // Otra semilla, misma partición y otras Pestañas.
        let otra = generar(&il, &Parametros { semilla: 1, ..p }).unwrap();
        assert_eq!(otra.etiquetas, r.etiquetas);
        assert_ne!(otra.uniones, r.uniones);
    }

    #[test]
    fn cuadricula_con_la_silueta_de_la_figura_se_ignora() {
        let p = Parametros {
            silueta_rectangular: false,
            ..en_cuadricula(1, 1)
        };
        let il = png_transparente(40, 40);
        // Sin figura falla por eso, no por la Cuadrícula de 1 × 1.
        let error = generar(&il, &p).unwrap_err().to_string();
        assert!(error.contains("ninguna figura"), "{error}");
    }

    #[test]
    fn cuadricula_fuera_de_rango_es_error() {
        let il = png_transparente(40, 40);
        for (filas, columnas) in [(1, 1), (0, 3), (9, 2), (2, 9)] {
            assert!(generar(&il, &en_cuadricula(filas, columnas)).is_err());
        }
        assert!(generar(&il, &en_cuadricula(1, 2)).is_ok());
    }

    #[test]
    fn cuadricula_avisa_si_las_esquinas_dejan_piezas_chicas() {
        let il = png_transparente(300, 300);
        let p = Parametros {
            esquinas_redondeadas: true,
            radio_esquinas_mm: 200.0,
            ..en_cuadricula(4, 4)
        };
        let aviso = generar(&il, &p).unwrap().aviso.unwrap();
        assert!(aviso.contains("radio de esquinas"), "{aviso}");
    }
}
