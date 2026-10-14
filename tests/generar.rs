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
    // El lienzo mide el Ancho más el Margen por lado, en mm (96 dpi para usvg).
    assert!((tree.size().width() as f64 - r.lienzo_mm.0 * 96.0 / 25.4).abs() < 0.5);
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
