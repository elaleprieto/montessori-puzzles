//! Ronda 05: exportación del Rompecabezas a SVG para láser.

use crate::geometria::{Punto, Rect, Segmento, Trazo};
use crate::mascara::{Formato, Ilustracion};
use base64::Engine;

/// Todo lo que va al archivo, ya en milímetros.
#[derive(Debug, Clone)]
pub struct Dibujo<'a> {
    pub ancho_mm: f64,
    pub alto_mm: f64,
    /// La Silueta y las Uniones: van a la Capa de corte.
    pub cortes: Vec<Trazo>,
    /// Va a la Capa de grabado, ubicada en `ilustracion_en`.
    pub ilustracion: &'a Ilustracion,
    /// Dónde va la Ilustración dentro del lienzo, en mm.
    pub ilustracion_en: Rect,
}

/// Genera el SVG con las capas `Corte` y `Grabado`.
///
/// Invariante: si la Ilustración es SVG, sus bytes ya parsean con usvg (lo
/// valida `Ilustracion::desde_bytes`). Si no parsearan, en debug se dispara un
/// `debug_assert!`; en release la capa `Grabado` queda vacía y el resto del
/// archivo sigue siendo válido.
pub fn exportar(dibujo: &Dibujo) -> String {
    let mut out = String::new();
    let (w, h) = (num(dibujo.ancho_mm), num(dibujo.alto_mm));
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" xmlns:inkscape=\"http://www.inkscape.org/namespaces/inkscape\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\">\n"
    ));
    out.push_str("<g id=\"grabado\" inkscape:groupmode=\"layer\" inkscape:label=\"Grabado\">\n");
    out.push_str(&grabado(dibujo));
    out.push_str("</g>\n");
    out.push_str("<g id=\"corte\" inkscape:groupmode=\"layer\" inkscape:label=\"Corte\">\n");
    for t in &dibujo.cortes {
        out.push_str(&format!(
            "<path fill=\"none\" stroke=\"#FF0000\" stroke-width=\"0.1\" d=\"{}\"/>\n",
            camino(t)
        ));
    }
    out.push_str("</g>\n</svg>\n");
    out
}

/// Número con hasta 3 decimales y sin ceros de más.
fn num(v: f64) -> String {
    debug_assert!(v.is_finite(), "número no finito en el SVG: {v}");
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

fn pt(p: Punto) -> String {
    format!("{} {}", num(p.x), num(p.y))
}

fn camino(t: &Trazo) -> String {
    let mut d = format!("M {}", pt(t.inicio));
    for seg in &t.segmentos {
        match seg {
            Segmento::Linea(p) => d.push_str(&format!(" L {}", pt(*p))),
            Segmento::Cubica(a, b, c) => {
                d.push_str(&format!(" C {} {} {}", pt(*a), pt(*b), pt(*c)))
            }
        }
    }
    d
}

fn grabado(dibujo: &Dibujo) -> String {
    let r = dibujo.ilustracion_en;
    let (x, y, w, h) = (num(r.x), num(r.y), num(r.ancho), num(r.alto));
    let ilu = dibujo.ilustracion;
    let mime = match ilu.formato {
        Formato::Png => "image/png",
        Formato::Jpg => "image/jpeg",
        Formato::Svg => return svg_anidado(&ilu.bytes, &x, &y, &w, &h),
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(&ilu.bytes);
    let uri = format!("data:{mime};base64,{b64}");
    format!(
        "<image x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" preserveAspectRatio=\"none\" href=\"{uri}\" xlink:href=\"{uri}\"/>\n"
    )
}

fn svg_anidado(bytes: &[u8], x: &str, y: &str, w: &str, h: &str) -> String {
    let tree = resvg::usvg::Tree::from_data(bytes, crate::mascara::opciones_svg());
    debug_assert!(tree.is_ok(), "la Ilustración SVG no parsea: {tree:?}");
    let Ok(tree) = tree else {
        return String::new();
    };
    let size = tree.size();
    // Prefija ids y referencias para que no choquen con las capas.
    let opciones = resvg::usvg::WriteOptions {
        id_prefix: Some("ilu-".into()),
        ..Default::default()
    };
    let texto = tree.to_string(&opciones);
    // Quita el prólogo y la etiqueta raíz: queda solo el contenido.
    let inicio = texto.find("<svg").unwrap_or(0);
    let abre = texto[inicio..].find('>').map_or(0, |i| inicio + i + 1);
    let cierra = texto.rfind("</svg>").unwrap_or(texto.len());
    let contenido = &texto[abre..cierra];
    format!(
        "<svg x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {} {}\" preserveAspectRatio=\"none\">{contenido}</svg>\n",
        num(size.width() as f64),
        num(size.height() as f64)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Punto {
        Punto::new(x, y)
    }

    fn png_chico() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(3, 2, image::Rgba([10, 200, 30, 255]));
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    fn ilu(bytes: Vec<u8>, formato: Formato) -> Ilustracion {
        Ilustracion {
            bytes,
            formato,
            ancho_px: 3,
            alto_px: 2,
        }
    }

    fn ilu_svg() -> Ilustracion {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="30" height="20" viewBox="0 0 30 20"><defs><linearGradient id="g"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient></defs><rect width="30" height="20" fill="url(#g)"/></svg>"##;
        Ilustracion {
            bytes: svg.to_vec(),
            formato: Formato::Svg,
            ancho_px: 30,
            alto_px: 20,
        }
    }

    fn dibujo<'a>(ilustracion: &'a Ilustracion, cortes: Vec<Trazo>) -> Dibujo<'a> {
        Dibujo {
            ancho_mm: 100.0,
            alto_mm: 60.5,
            cortes,
            ilustracion,
            ilustracion_en: Rect {
                x: 5.0,
                y: 6.25,
                ancho: 90.0,
                alto: 48.0,
            },
        }
    }

    fn trazo_simple() -> Trazo {
        Trazo {
            inicio: p(1.0, 2.0),
            segmentos: vec![
                Segmento::Linea(p(3.5, 4.0)),
                Segmento::Cubica(p(5.0, 6.0), p(7.125, 8.0), p(9.0, 10.0)),
            ],
        }
    }

    fn parsea(s: &str) {
        resvg::usvg::Tree::from_data(s.as_bytes(), &resvg::usvg::Options::default())
            .expect("SVG mal formado");
    }

    #[test]
    fn resultado_bien_formado_con_png_y_con_svg() {
        let png = ilu(png_chico(), Formato::Png);
        parsea(&exportar(&dibujo(&png, vec![trazo_simple()])));
        let svg = ilu_svg();
        parsea(&exportar(&dibujo(&svg, vec![trazo_simple()])));
    }

    #[test]
    fn tamano_en_mm_y_viewbox_coinciden() {
        let png = ilu(png_chico(), Formato::Png);
        let s = exportar(&dibujo(&png, vec![]));
        assert!(s.contains(r#"width="100mm""#));
        assert!(s.contains(r#"height="60.5mm""#));
        assert!(s.contains(r#"viewBox="0 0 100 60.5""#));
    }

    #[test]
    fn corte_tiene_un_path_por_trazo_con_estilo_de_hairline() {
        let png = ilu(png_chico(), Formato::Png);
        let s = exportar(&dibujo(
            &png,
            vec![trazo_simple(), trazo_simple(), trazo_simple()],
        ));
        let corte = &s[s.find(r#"id="corte""#).unwrap()..];
        assert_eq!(corte.matches("<path").count(), 3);
        assert_eq!(corte.matches(r##"stroke="#FF0000""##).count(), 3);
        assert_eq!(corte.matches(r#"stroke-width="0.1""#).count(), 3);
        assert_eq!(corte.matches(r#"fill="none""#).count(), 3);
    }

    #[test]
    fn formato_de_d() {
        let png = ilu(png_chico(), Formato::Png);
        let s = exportar(&dibujo(&png, vec![trazo_simple()]));
        assert!(s.contains(r#"d="M 1 2 L 3.5 4 C 5 6 7.125 8 9 10""#), "{s}");
    }

    #[test]
    fn numeros_con_tres_decimales_sin_ceros_de_mas() {
        let png = ilu(png_chico(), Formato::Png);
        let t = Trazo {
            inicio: p(1.23456, -0.0001),
            segmentos: vec![Segmento::Linea(p(12.5, 2.0004))],
        };
        let s = exportar(&dibujo(&png, vec![t]));
        assert!(s.contains(r#"d="M 1.235 0 L 12.5 2""#), "{s}");
    }

    #[test]
    fn png_se_incrusta_con_sus_bytes_originales() {
        let bytes = png_chico();
        let png = ilu(bytes.clone(), Formato::Png);
        let s = exportar(&dibujo(&png, vec![]));
        let marca = "data:image/png;base64,";
        let i = s.find(marca).unwrap() + marca.len();
        let b64 = &s[i..i + s[i..].find('"').unwrap()];
        let dec = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        assert_eq!(dec, bytes);
        assert!(s.contains(r#"preserveAspectRatio="none""#));
        assert!(s.contains(r#"x="5" y="6.25" width="90" height="48""#));
        assert!(s.contains("xlink:href=\"data:image/png"));
        assert!(s.contains(" href=\"data:image/png"));
    }

    #[test]
    fn jpg_usa_mime_jpeg() {
        let jpg = ilu(vec![0xFF, 0xD8, 0xFF, 0xD9], Formato::Jpg);
        let s = exportar(&dibujo(&jpg, vec![]));
        assert!(s.contains("data:image/jpeg;base64,"));
        assert!(!s.contains("image/png"));
    }

    #[test]
    fn svg_se_incrusta_como_vector_anidado() {
        let svg = ilu_svg();
        let s = exportar(&dibujo(&svg, vec![]));
        assert!(!s.contains("<image"));
        assert_eq!(s.matches("<?xml").count(), 0);
        let grabado = &s[s.find(r#"id="grabado""#).unwrap()..s.find(r#"id="corte""#).unwrap()];
        let i = grabado.find("<svg").expect("sin svg anidado");
        let tag = &grabado[i..i + grabado[i..].find('>').unwrap()];
        assert!(tag.contains(r#"x="5""#), "{tag}");
        assert!(tag.contains(r#"y="6.25""#), "{tag}");
        assert!(tag.contains(r#"width="90""#), "{tag}");
        assert!(tag.contains(r#"height="48""#), "{tag}");
        assert!(tag.contains(r#"viewBox="0 0 30 20""#), "{tag}");
        assert!(tag.contains(r#"preserveAspectRatio="none""#), "{tag}");
        // Lo que sigue al tag de apertura es contenido, sin restos de la raíz.
        let resto = grabado[i + tag.len() + 1..].trim_start();
        assert!(resto.starts_with('<'), "{resto}");
        assert!(!resto.starts_with("<svg"), "{resto}");
    }

    #[test]
    fn ids_del_svg_incrustado_no_chocan_con_las_capas() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><defs><linearGradient id="corte"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient></defs><rect width="10" height="10" fill="url(#corte)"/></svg>"##;
        let ilustracion = Ilustracion {
            bytes: svg.to_vec(),
            formato: Formato::Svg,
            ancho_px: 10,
            alto_px: 10,
        };
        let s = exportar(&dibujo(&ilustracion, vec![]));
        assert_eq!(s.matches(r#"id="corte""#).count(), 1, "{s}");
        assert_eq!(s.matches(r#"id="grabado""#).count(), 1);
        assert!(s.contains("url(#ilu-"), "{s}");
        assert!(!s.contains("url(#corte)"), "{s}");
        parsea(&s);
    }

    #[test]
    fn ids_ya_prefijados_no_se_duplican() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="30" height="20"><clipPath id="g"><rect width="5" height="5"/></clipPath><clipPath id="ilu-g"><rect width="9" height="9"/></clipPath><rect width="30" height="20" clip-path="url(#g)"/><rect width="30" height="20" clip-path="url(#ilu-g)"/></svg>"##;
        let ilustracion = Ilustracion {
            bytes: svg.to_vec(),
            formato: Formato::Svg,
            ancho_px: 30,
            alto_px: 20,
        };
        let s = exportar(&dibujo(&ilustracion, vec![]));
        assert_eq!(s.matches(r#"id="ilu-g""#).count(), 1, "{s}");
        assert_eq!(s.matches(r#"id="ilu-ilu-g""#).count(), 1, "{s}");
        assert_eq!(s.matches("url(#ilu-g)").count(), 1, "{s}");
        assert_eq!(s.matches("url(#ilu-ilu-g)").count(), 1, "{s}");
        parsea(&s);
    }

    /// En debug salta el `debug_assert!`; en release la capa queda vacía.
    #[test]
    #[cfg_attr(debug_assertions, should_panic(expected = "no parsea"))]
    fn svg_que_no_parsea_deja_el_grabado_vacio() {
        let ilustracion = Ilustracion {
            bytes: b"no es svg".to_vec(),
            formato: Formato::Svg,
            ancho_px: 1,
            alto_px: 1,
        };
        let s = exportar(&dibujo(&ilustracion, vec![trazo_simple()]));
        assert!(s.contains(
            "<g id=\"grabado\" inkscape:groupmode=\"layer\" inkscape:label=\"Grabado\">\n</g>"
        ));
        parsea(&s);
    }

    #[test]
    fn grabado_va_antes_que_corte_y_son_capas_de_inkscape() {
        let png = ilu(png_chico(), Formato::Png);
        let s = exportar(&dibujo(&png, vec![trazo_simple()]));
        assert!(s.find(r#"id="grabado""#).unwrap() < s.find(r#"id="corte""#).unwrap());
        assert!(s.contains(r#"inkscape:groupmode="layer" inkscape:label="Grabado""#));
        assert!(s.contains(r#"inkscape:groupmode="layer" inkscape:label="Corte""#));
        assert!(s.contains(r#"xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape""#));
    }
}
