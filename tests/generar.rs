use montessori_puzzles::Parametros;
use montessori_puzzles::geometria::{Rect, Segmento, Trazo};
use montessori_puzzles::mascara::{Formato, Ilustracion};
use montessori_puzzles::particion::FONDO;
use montessori_puzzles::rompecabezas::{Rompecabezas, exportar_svg, generar};
use std::collections::BTreeSet;

/// Un "elefante": cuerpo elíptico, 4 patas y una trompa curva, sobre fondo transparente.
fn elefante() -> Ilustracion {
    let (w, h) = (400u32, 300u32);
    let mut img = image::RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f64, y as f64);
            let cuerpo = ((fx - 200.0) / 130.0).powi(2) + ((fy - 130.0) / 80.0).powi(2) <= 1.0;
            let pata = |x0: f64| (x0..x0 + 30.0).contains(&fx) && (190.0..270.0).contains(&fy);
            let patas = pata(100.0) || pata(150.0) || pata(220.0) || pata(270.0);
            // Trompa: arco de anillo a la izquierda del cuerpo.
            let d = ((fx - 85.0).powi(2) + (fy - 170.0).powi(2)).sqrt();
            let trompa = (40.0..58.0).contains(&d) && fx < 85.0 && fy > 100.0;
            if cuerpo || patas || trompa {
                img.put_pixel(x, y, image::Rgba([120, 120, 130, 255]));
            }
        }
    }
    png(&img)
}

fn png(img: &image::RgbaImage) -> Ilustracion {
    let mut b = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut b), image::ImageFormat::Png)
        .unwrap();
    Ilustracion::desde_bytes(b, Formato::Png).unwrap()
}

fn caja(trazos: &[Trazo]) -> (f64, f64, f64, f64) {
    let mut c = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for t in trazos {
        let mut pts = vec![t.inicio];
        for s in &t.segmentos {
            match s {
                Segmento::Linea(p) => pts.push(*p),
                Segmento::Cubica(_, _, p) => pts.push(*p),
            }
        }
        for p in pts {
            c = (c.0.min(p.x), c.1.min(p.y), c.2.max(p.x), c.3.max(p.y));
        }
    }
    c
}

fn con_cubicas(t: &Trazo) -> bool {
    t.segmentos
        .iter()
        .any(|s| matches!(s, Segmento::Cubica(..)))
}

#[test]
fn elefante_con_cinco_piezas() {
    let il = elefante();
    let r = generar(&il, &Parametros::default()).unwrap();
    let piezas: BTreeSet<u16> = r
        .etiquetas
        .datos
        .iter()
        .copied()
        .filter(|&e| e != FONDO)
        .collect();
    assert_eq!(piezas, (1..=5).collect());
    assert!(!r.silueta.is_empty());
    assert!(r.uniones.len() >= 4);
    assert!(r.mm_por_px > 0.0);
    assert!(r.lienzo_mm.0 >= 200.0);
    let svg = exportar_svg(&r, &il);
    let tree =
        resvg::usvg::Tree::from_data(svg.as_bytes(), &resvg::usvg::Options::default()).unwrap();
    // La página se ajusta a la caja de la Silueta, en mm (96 dpi para usvg).
    let (x0, y0, x1, y1) = caja(&r.silueta);
    assert!((tree.size().width() as f64 - (x1 - x0) * 96.0 / 25.4).abs() < 0.5);
    assert!((tree.size().height() as f64 - (y1 - y0) * 96.0 / 25.4).abs() < 0.5);
    assert!(svg.contains("mm\" height=\""));
}

#[test]
fn lienzo_e_ilustracion_en_milimetros() {
    let il = elefante();
    let r = generar(&il, &Parametros::default()).unwrap();
    let Rect { x, y, ancho, alto } = r.ilustracion_en;
    assert!((ancho - 200.0).abs() < 1e-9);
    assert!((alto - 150.0).abs() < 1.0, "alto {alto}");
    assert!(x > 0.0 && y > 0.0);
    assert!((r.lienzo_mm.0 - (ancho + 2.0 * x)).abs() < 1e-6);
    assert!((r.lienzo_mm.1 - (alto + 2.0 * y)).abs() < 1e-6);
    // Toda la geometría cae dentro del lienzo.
    let todo: Vec<Trazo> = r.silueta.iter().chain(&r.uniones).cloned().collect();
    let (x0, y0, x1, y1) = caja(&todo);
    assert!(x0 >= 0.0 && y0 >= 0.0 && x1 <= r.lienzo_mm.0 && y1 <= r.lienzo_mm.1);
}

#[test]
fn validacion_de_parametros() {
    let il = elefante();
    for piezas in [1, 13] {
        let e = generar(
            &il,
            &Parametros {
                piezas,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(e.to_string().contains("piezas"), "{e}");
    }
    let e = generar(
        &il,
        &Parametros {
            ancho_mm: 0.0,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(e.to_string().contains("ancho"), "{e}");
}

#[test]
fn ilustracion_transparente_pide_ajustar_el_umbral() {
    let il = png(&image::RgbaImage::new(50, 50));
    let e = generar(&il, &Parametros::default()).unwrap_err();
    assert!(e.to_string().contains("Umbral"), "{e}");
}

#[test]
fn mayor_margen_aleja_la_silueta() {
    let il = elefante();
    let ancho = |margen_mm| {
        let r = generar(
            &il,
            &Parametros {
                margen_mm,
                ..Default::default()
            },
        )
        .unwrap();
        let c = caja(&r.silueta);
        c.2 - c.0
    };
    let (a0, a3) = (ancho(0.0), ancho(3.0));
    assert!(a3 > a0 + 4.0, "margen 3: {a3}, margen 0: {a0}");
}

#[test]
fn uniones_cortas_no_llevan_pestana() {
    let il = elefante();
    let r = generar(
        &il,
        &Parametros {
            ancho_mm: 40.0,
            piezas: 8,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!r.uniones.is_empty());
    for u in &r.uniones {
        let mut largo = 0.0;
        let mut ant = u.inicio;
        for s in &u.segmentos {
            let p = match s {
                Segmento::Linea(p) | Segmento::Cubica(_, _, p) => *p,
            };
            largo += (p.x - ant.x).hypot(p.y - ant.y);
            ant = p;
        }
        if largo < 15.0 {
            assert!(!con_cubicas(u), "Unión de {largo} mm con cúbicas");
        }
    }
}

#[test]
fn uniones_largas_llevan_pestana() {
    let il = elefante();
    let r = generar(&il, &Parametros::default()).unwrap();
    assert!(r.uniones.iter().any(con_cubicas));
}

#[test]
fn la_silueta_deja_relleno_aun_con_margen_cero() {
    // Figura que toca los bordes izquierdo, superior e inferior de la Ilustración:
    // la Silueta no debe pegarse al borde del lienzo.
    let mut img = image::RgbaImage::new(100, 100);
    for y in 0..100 {
        for x in 0..60 {
            img.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
        }
    }
    let il = png(&img);
    let r = generar(
        &il,
        &Parametros {
            margen_mm: 0.0,
            ..Default::default()
        },
    )
    .unwrap();
    let (x0, y0, x1, y1) = caja(&r.silueta);
    let m = r.mm_por_px;
    assert!(x0 >= m && y0 >= m, "{x0} {y0}");
    assert!(x1 <= r.lienzo_mm.0 - m && y1 <= r.lienzo_mm.1 - m);
}

#[test]
fn determinista() {
    let il = elefante();
    let p = Parametros {
        semilla: 7,
        ..Default::default()
    };
    let a = exportar_svg(&generar(&il, &p).unwrap(), &il);
    let b = exportar_svg(&generar(&il, &p).unwrap(), &il);
    assert_eq!(a, b);
}

#[test]
fn la_semilla_usada_se_reporta() {
    let il = elefante();
    let r: Rompecabezas = generar(
        &il,
        &Parametros {
            semilla: 41,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(r.semilla_usada >= 41 && r.semilla_usada < 61);
}

#[test]
fn avisa_si_la_ilustracion_usa_el_rojo_de_corte() {
    let svg = |trazo: &str| {
        let s = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80"><ellipse cx="50" cy="40" rx="45" ry="35" fill="#888" stroke="{trazo}" stroke-width="2"/></svg>"##
        );
        Ilustracion::desde_bytes(s.into_bytes(), Formato::Svg).unwrap()
    };
    let con = generar(&svg("red"), &Parametros::default()).unwrap();
    assert!(con.aviso.unwrap().contains("#FF0000"));
    let sin = generar(&svg("#0000ff"), &Parametros::default()).unwrap();
    assert!(sin.aviso.is_none(), "{:?}", sin.aviso);
}

/// Recorre `fixtures/` si existe; escribe en `target/fixtures-out/`.
#[test]
#[ignore = "necesita la carpeta fixtures/"]
fn recorre_fixtures() {
    let Ok(dir) = std::fs::read_dir("fixtures") else {
        return;
    };
    std::fs::create_dir_all("target/fixtures-out").unwrap();
    for e in dir.flatten() {
        let ruta = e.path();
        let formato = match ruta
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .as_deref()
        {
            Some("png") => Formato::Png,
            Some("jpg" | "jpeg") => Formato::Jpg,
            Some("svg") => Formato::Svg,
            _ => continue,
        };
        let il = Ilustracion::desde_bytes(std::fs::read(&ruta).unwrap(), formato).unwrap();
        let r = generar(&il, &Parametros::default()).unwrap();
        let nombre = ruta.file_stem().unwrap().to_string_lossy();
        std::fs::write(
            format!("target/fixtures-out/{nombre}.svg"),
            exportar_svg(&r, &il),
        )
        .unwrap();
    }
}

/// La imagen embebida en la Capa de grabado: (mime, bytes).
fn grabado_embebido(svg: &str) -> (String, Vec<u8>) {
    use base64::Engine;
    let ini = svg.find("href=\"data:").expect("sin imagen embebida") + "href=\"data:".len();
    let resto = &svg[ini..];
    let (mime, resto) = resto.split_once(";base64,").unwrap();
    let datos = &resto[..resto.find('"').unwrap()];
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(datos)
        .unwrap();
    (mime.to_string(), bytes)
}

#[test]
fn el_grabado_de_un_jpg_con_fondo_liso_no_lleva_el_fondo() {
    // Fondo blanco opaco, un disco oscuro (la figura) y una marca suelta lejos
    // (como la línea punteada de "recortar" de muchas ilustraciones).
    let (w, h) = (300u32, 200u32);
    let mut img = image::RgbImage::from_pixel(w, h, image::Rgb([255, 255, 255]));
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f64 - 150.0).powi(2) + (y as f64 - 100.0).powi(2)).sqrt();
            let marca = (10..20).contains(&x) && (10..20).contains(&y);
            if d <= 70.0 || marca {
                img.put_pixel(x, y, image::Rgb([40, 60, 160]));
            }
        }
    }
    let mut b = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut b), image::ImageFormat::Jpeg)
        .unwrap();
    let il = Ilustracion::desde_bytes(b, Formato::Jpg).unwrap();
    let p = Parametros {
        piezas: 3,
        ancho_mm: 150.0,
        ..Default::default()
    };
    let r = generar(&il, &p).unwrap();
    let svg = exportar_svg(&r, &il);
    let (mime, bytes) = grabado_embebido(&svg);

    assert_eq!(mime, "image/png");
    let g = image::load_from_memory(&bytes).unwrap().to_rgba8();
    let (gw, gh) = g.dimensions();
    // Recortada a la figura (disco de 140 px + Margen), no la Ilustración entera.
    assert!(gw < 160 && gh < 160, "recortada: {gw}×{gh}");
    for (x, y) in [(0, 0), (gw - 1, 0), (0, gh - 1), (gw - 1, gh - 1)] {
        assert_eq!(g.get_pixel(x, y)[3], 0, "fuera del disco no se graba");
    }
    let centro = g.get_pixel(gw / 2, gh / 2);
    assert_eq!(centro[3], 255, "la figura se graba");
    assert!(
        centro[2] > 120 && centro[0] < 90,
        "con su color: {centro:?}"
    );
    // Y ocupa toda la página, que es la caja de la Silueta.
    let (x0, y0, x1, y1) = caja(&r.silueta);
    let (ix, iy, iw, ih) = rect_de_imagen(&svg);
    let pagina = |n: &str| -> f64 {
        let i = svg.find(&format!("{n}=\"")).unwrap() + n.len() + 2;
        svg[i..i + svg[i..].find("mm").unwrap()].parse().unwrap()
    };
    assert!((pagina("width") - (x1 - x0)).abs() < 1e-3);
    assert!((pagina("height") - (y1 - y0)).abs() < 1e-3);
    for (a, b) in [(ix, 0.0), (iy, 0.0), (iw, x1 - x0), (ih, y1 - y0)] {
        assert!(
            (a - b).abs() < 1.0,
            "imagen {ix} {iy} {iw} {ih} vs silueta {x0} {y0} {x1} {y1}"
        );
    }
}

/// x, y, ancho y alto (mm) del `<image>` de la Capa de grabado.
fn rect_de_imagen(svg: &str) -> (f64, f64, f64, f64) {
    let tag = &svg[svg.find("<image").unwrap()..];
    let attr = |n: &str| -> f64 {
        let i = tag.find(&format!(" {n}=\"")).unwrap() + n.len() + 3;
        tag[i..i + tag[i..].find('"').unwrap()].parse().unwrap()
    };
    (attr("x"), attr("y"), attr("width"), attr("height"))
}

#[test]
fn el_grabado_de_un_svg_va_tal_cual() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect width="100" height="100" fill="white"/><circle cx="50" cy="50" r="30" fill="black"/></svg>"#;
    let il = Ilustracion::desde_bytes(svg.to_vec(), Formato::Svg).unwrap();
    let r = generar(&il, &Parametros::default()).unwrap();
    // Sigue vectorial (usvg lo normaliza a <path>), sin pasar a imagen.
    assert!(!exportar_svg(&r, &il).contains("<image"));
}
