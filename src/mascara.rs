//! Ronda 01: de la Ilustración a la máscara de la figura.

use anyhow::{Context, Result, bail};
use resvg::{tiny_skia, usvg};
use std::collections::VecDeque;

/// Formato de archivo de una Ilustración.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Formato {
    Png,
    Jpg,
    Svg,
}

/// La imagen que carga el usuario, tal cual vino del archivo.
#[derive(Debug, Clone)]
pub struct Ilustracion {
    pub bytes: Vec<u8>,
    pub formato: Formato,
    /// Tamaño intrínseco en píxeles (para SVG, el de su viewBox).
    pub ancho_px: u32,
    pub alto_px: u32,
}

impl Ilustracion {
    /// Decodifica lo justo para validar el archivo y conocer su tamaño.
    pub fn desde_bytes(bytes: Vec<u8>, formato: Formato) -> Result<Self> {
        let (ancho_px, alto_px) = match decodificar(&bytes, formato)? {
            Decodificada::Raster(img) => (img.width(), img.height()),
            Decodificada::Vector(tree) => {
                let t = tree.size();
                (t.width().round() as u32, t.height().round() as u32)
            }
        };
        if ancho_px == 0 || alto_px == 0 {
            bail!("La ilustración tiene tamaño cero");
        }
        Ok(Self {
            bytes,
            formato,
            ancho_px,
            alto_px,
        })
    }
}

/// Opciones de usvg compartidas por toda la app: cargan las fuentes del
/// sistema una sola vez para que el `<text>` se convierta en trazos.
pub fn opciones_svg() -> &'static usvg::Options<'static> {
    static OPCIONES: std::sync::OnceLock<usvg::Options<'static>> = std::sync::OnceLock::new();
    OPCIONES.get_or_init(|| {
        let mut o = usvg::Options::default();
        o.fontdb_mut().load_system_fonts();
        o
    })
}

/// Contenido decodificado de una Ilustración según su Formato.
enum Decodificada {
    Raster(image::DynamicImage),
    Vector(Box<usvg::Tree>),
}

fn decodificar(bytes: &[u8], formato: Formato) -> Result<Decodificada> {
    let raster = |f| {
        image::load_from_memory_with_format(bytes, f)
            .map(Decodificada::Raster)
            .context(
                "No se pudo leer la imagen: el archivo está dañado o no es del formato indicado",
            )
    };
    match formato {
        Formato::Png => raster(image::ImageFormat::Png),
        Formato::Jpg => raster(image::ImageFormat::Jpeg),
        Formato::Svg => usvg::Tree::from_data(bytes, opciones_svg())
            .map(|t| Decodificada::Vector(Box::new(t)))
            .context("No se pudo leer el SVG: el archivo está dañado o no es un SVG válido"),
    }
}

/// Conexidad de una inundación: 4 vecinos (lados) u 8 (lados y diagonales).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Conexidad {
    Cuatro,
    Ocho,
}

/// Rasteriza la Ilustración a RGBA con su lado mayor en `lado_max` px.
pub fn rasterizar(ilustracion: &Ilustracion, lado_max: usize) -> Result<image::RgbaImage> {
    if lado_max == 0 {
        bail!("El lado máximo debe ser mayor que cero");
    }
    let (w, h) = (ilustracion.ancho_px as f64, ilustracion.alto_px as f64);
    let escala = lado_max as f64 / w.max(h);
    let nw = ((w * escala).round() as u32).max(1);
    let nh = ((h * escala).round() as u32).max(1);
    match decodificar(&ilustracion.bytes, ilustracion.formato)? {
        Decodificada::Raster(img) => Ok(image::imageops::resize(
            &img.to_rgba8(),
            nw,
            nh,
            image::imageops::FilterType::Triangle,
        )),
        Decodificada::Vector(tree) => {
            let mut pixmap = tiny_skia::Pixmap::new(nw, nh).context("Tamaño de raster inválido")?;
            let t = tree.size();
            let transform =
                tiny_skia::Transform::from_scale(nw as f32 / t.width(), nh as f32 / t.height());
            resvg::render(&tree, transform, &mut pixmap.as_mut());
            let mut out = image::RgbaImage::new(nw, nh);
            for (i, px) in pixmap.pixels().iter().enumerate() {
                let c = px.demultiply();
                out.put_pixel(
                    i as u32 % nw,
                    i as u32 / nw,
                    image::Rgba([c.red(), c.green(), c.blue(), c.alpha()]),
                );
            }
            Ok(out)
        }
    }
}

/// Grilla binaria de trabajo: `true` es figura, `false` es fondo.
/// Se indexa por fila: `datos[y * ancho + x]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mascara {
    pub ancho: usize,
    pub alto: usize,
    pub datos: Vec<bool>,
}

impl Mascara {
    pub fn vacia(ancho: usize, alto: usize) -> Self {
        Self {
            ancho,
            alto,
            datos: vec![false; ancho * alto],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> bool {
        self.datos[y * self.ancho + x]
    }

    pub fn set(&mut self, x: usize, y: usize, valor: bool) {
        self.datos[y * self.ancho + x] = valor;
    }

    /// Rasteriza la Ilustración con su lado mayor en `lado_max` px, la ubica
    /// a `relleno_px` del borde (la grilla mide la Ilustración + 2 × relleno
    /// por lado) y separa la figura del fondo según el Umbral (0–255).
    pub fn desde_ilustracion(
        ilustracion: &Ilustracion,
        umbral: u8,
        lado_max: usize,
        relleno_px: usize,
    ) -> Result<Self> {
        let img = rasterizar(ilustracion, lado_max)?;
        let (w, h) = (img.width() as usize, img.height() as usize);
        let opacidad = opacidades(&img);
        let mut m = Self::vacia(w + 2 * relleno_px, h + 2 * relleno_px);
        for y in 0..h {
            for x in 0..w {
                m.set(
                    x + relleno_px,
                    y + relleno_px,
                    opacidad[y * w + x] >= umbral,
                );
            }
        }
        Ok(m)
    }

    /// Se queda con la componente conexa más grande y rellena sus huecos.
    pub fn figura_principal(&self) -> Self {
        let n = self.datos.len();
        // Componentes 4-conexas de figura: nos quedamos con la más grande.
        let mut visto = vec![false; n];
        let mut mejor: Vec<usize> = Vec::new();
        for ini in 0..n {
            if !self.datos[ini] || visto[ini] {
                continue;
            }
            let comp = self.inundar(ini, true, Conexidad::Cuatro, &mut visto);
            if comp.len() > mejor.len() {
                mejor = comp;
            }
        }
        let mut out = Self::vacia(self.ancho, self.alto);
        for &i in &mejor {
            out.datos[i] = true;
        }
        // Fondo 8-conexo alcanzable desde el borde; el resto es hueco.
        let mut fuera = vec![false; n];
        for y in 0..self.alto {
            for x in 0..self.ancho {
                let borde = x == 0 || y == 0 || x + 1 == self.ancho || y + 1 == self.alto;
                let i = y * self.ancho + x;
                if borde && !out.datos[i] && !fuera[i] {
                    out.inundar(i, false, Conexidad::Ocho, &mut fuera);
                }
            }
        }
        for (d, &f) in out.datos.iter_mut().zip(&fuera) {
            if !*d && !f {
                *d = true;
            }
        }
        out
    }

    /// Dilata la figura `radio_px` píxeles (el Margen ya convertido a píxeles).
    pub fn dilatar(&self, radio_px: f64) -> Self {
        if radio_px.is_nan() || radio_px <= 0.0 {
            return self.clone();
        }
        let r = radio_px.floor() as i64;
        let r2 = radio_px * radio_px;
        let mut offsets = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                if ((dx * dx + dy * dy) as f64) <= r2 {
                    offsets.push((dx, dy));
                }
            }
        }
        // Basta dilatar desde el contorno (figura con algún 4-vecino de fondo
        // o pegada al borde de la grilla): el píxel de figura más cercano a
        // uno de fondo siempre es de contorno, así que el resultado es exacto.
        let mut out = self.clone();
        for y in 0..self.alto {
            for x in 0..self.ancho {
                if !self.es_contorno(x, y) {
                    continue;
                }
                for &(dx, dy) in &offsets {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < self.ancho && (ny as usize) < self.alto
                    {
                        out.set(nx as usize, ny as usize, true);
                    }
                }
            }
        }
        out
    }

    /// Figura que toca fondo por un lado (o el borde de la grilla).
    fn es_contorno(&self, x: usize, y: usize) -> bool {
        self.get(x, y)
            && (x == 0
                || y == 0
                || x + 1 == self.ancho
                || y + 1 == self.alto
                || !self.get(x - 1, y)
                || !self.get(x + 1, y)
                || !self.get(x, y - 1)
                || !self.get(x, y + 1))
    }

    /// Inunda desde `ini` los píxeles con valor `valor`; devuelve los índices.
    fn inundar(
        &self,
        ini: usize,
        valor: bool,
        conexidad: Conexidad,
        visto: &mut [bool],
    ) -> Vec<usize> {
        let mut comp = Vec::new();
        let mut cola = VecDeque::from([ini]);
        visto[ini] = true;
        while let Some(i) = cola.pop_front() {
            comp.push(i);
            let (x, y) = ((i % self.ancho) as i64, (i / self.ancho) as i64);
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    if (dx == 0 && dy == 0)
                        || (conexidad == Conexidad::Cuatro && dx != 0 && dy != 0)
                    {
                        continue;
                    }
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= self.ancho as i64 || ny >= self.alto as i64 {
                        continue;
                    }
                    let j = ny as usize * self.ancho + nx as usize;
                    if !visto[j] && self.datos[j] == valor {
                        visto[j] = true;
                        cola.push_back(j);
                    }
                }
            }
        }
        comp
    }
}

/// Opacidad 0–255 por píxel: alpha si hay transparencia; si no, la distancia
/// de color al fondo (mediana por canal del borde) amplificada ×4.
fn opacidades(img: &image::RgbaImage) -> Vec<u8> {
    let (w, h) = (img.width(), img.height());
    if img.pixels().any(|p| p[3] < 255) {
        return img.pixels().map(|p| p[3]).collect();
    }
    let mut canales: [Vec<u8>; 3] = Default::default();
    for y in 0..h {
        for x in 0..w {
            if x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                let p = img.get_pixel(x, y);
                for (c, v) in canales.iter_mut().zip(p.0) {
                    c.push(v);
                }
            }
        }
    }
    let mut fondo = [0u8; 3];
    for (f, c) in fondo.iter_mut().zip(canales.iter_mut()) {
        c.sort_unstable();
        *f = c[c.len() / 2];
    }
    img.pixels()
        .map(|p| {
            let d = (0..3).map(|k| p[k].abs_diff(fondo[k])).max().unwrap_or(0);
            (4 * d as u32).min(255) as u8
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use std::io::Cursor;

    /// Requiere alguna fuente instalada en el sistema.
    #[test]
    #[ignore = "necesita fuentes del sistema"]
    fn el_texto_de_un_svg_es_figura() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><text x="5" y="85" font-size="90" font-family="sans-serif" font-weight="bold">MM</text></svg>"#;
        let il = Ilustracion::desde_bytes(svg.to_vec(), Formato::Svg).unwrap();
        let m = Mascara::desde_ilustracion(&il, 128, 200, 0).unwrap();
        assert!(m.datos.iter().filter(|&&b| b).count() > 500);
    }

    fn codificar(img: &RgbaImage, fmt: ImageFormat) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        let din = if fmt == ImageFormat::Jpeg {
            DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(img.clone()).to_rgb8())
        } else {
            DynamicImage::ImageRgba8(img.clone())
        };
        din.write_to(&mut out, fmt).unwrap();
        out.into_inner()
    }

    fn png(img: &RgbaImage) -> Ilustracion {
        Ilustracion::desde_bytes(codificar(img, ImageFormat::Png), Formato::Png).unwrap()
    }

    fn svg(texto: &str) -> Ilustracion {
        Ilustracion::desde_bytes(texto.as_bytes().to_vec(), Formato::Svg).unwrap()
    }

    const SVG_CIRCULO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20" viewBox="0 0 40 20"><circle cx="20" cy="10" r="8" fill="black"/></svg>"#;

    /// Fondo `fondo` con un círculo negro centrado de radio `r`.
    fn circulo_sobre(fondo: Rgba<u8>, lado: u32, r: f64) -> RgbaImage {
        let mut img = RgbaImage::from_pixel(lado, lado, fondo);
        let c = lado as f64 / 2.0;
        for y in 0..lado {
            for x in 0..lado {
                let d = ((x as f64 + 0.5 - c).powi(2) + (y as f64 + 0.5 - c).powi(2)).sqrt();
                if d <= r {
                    img.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                }
            }
        }
        img
    }

    fn mascara_de(filas: &[&str]) -> Mascara {
        let mut m = Mascara::vacia(filas[0].len(), filas.len());
        for (y, f) in filas.iter().enumerate() {
            for (x, c) in f.chars().enumerate() {
                m.set(x, y, c == '#');
            }
        }
        m
    }

    fn cuenta(m: &Mascara) -> usize {
        m.datos.iter().filter(|&&b| b).count()
    }

    #[test]
    fn desde_bytes_lee_tamano_de_png_jpg_y_svg() {
        let img = RgbaImage::from_pixel(30, 10, Rgba([255, 255, 255, 255]));
        let p = png(&img);
        assert_eq!((p.ancho_px, p.alto_px), (30, 10));
        let j = Ilustracion::desde_bytes(codificar(&img, ImageFormat::Jpeg), Formato::Jpg).unwrap();
        assert_eq!((j.ancho_px, j.alto_px), (30, 10));
        let s = svg(SVG_CIRCULO);
        assert_eq!((s.ancho_px, s.alto_px), (40, 20));
    }

    #[test]
    fn desde_bytes_rechaza_basura() {
        for f in [Formato::Png, Formato::Jpg, Formato::Svg] {
            let e = Ilustracion::desde_bytes(b"esto no es una imagen".to_vec(), f);
            assert!(e.is_err());
        }
    }

    #[test]
    fn rasterizar_png_conserva_proporcion() {
        let img = RgbaImage::from_pixel(30, 10, Rgba([255, 0, 0, 255]));
        let r = rasterizar(&png(&img), 60).unwrap();
        assert_eq!((r.width(), r.height()), (60, 20));
        let r = rasterizar(&png(&RgbaImage::from_pixel(10, 30, Rgba([0; 4]))), 15).unwrap();
        assert_eq!((r.width(), r.height()), (5, 15));
    }

    #[test]
    fn rasterizar_svg_dibuja_circulo_sobre_transparente() {
        let r = rasterizar(&svg(SVG_CIRCULO), 80).unwrap();
        assert_eq!((r.width(), r.height()), (80, 40));
        assert_eq!(r.get_pixel(40, 20)[3], 255);
        assert_eq!(r.get_pixel(40, 20)[0], 0);
        assert_eq!(r.get_pixel(1, 1)[3], 0);
    }

    #[test]
    fn rasterizar_svg_desmultiplica_el_alpha() {
        let s = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff8000" fill-opacity="0.5"/></svg>"##;
        let p = *rasterizar(&svg(s), 10).unwrap().get_pixel(5, 5);
        assert!(p[3].abs_diff(128) <= 1, "{p:?}");
        assert!(p[0] >= 254 && p[1].abs_diff(128) <= 2 && p[2] == 0, "{p:?}");
    }

    #[test]
    fn alpha_cuadrado_opaco_es_exactamente_la_figura_con_relleno() {
        let mut img = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 0]));
        for y in 3..7 {
            for x in 2..8 {
                img.put_pixel(x, y, Rgba([10, 20, 30, 255]));
            }
        }
        let m = Mascara::desde_ilustracion(&png(&img), 128, 10, 5).unwrap();
        assert_eq!((m.ancho, m.alto), (20, 20));
        for y in 0..20 {
            for x in 0..20 {
                let esperado = (7..13).contains(&x) && (8..12).contains(&y);
                assert_eq!(m.get(x, y), esperado, "({x},{y})");
            }
        }
    }

    #[test]
    fn umbral_decide_sobre_pixel_semitransparente() {
        let mut img = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 0]));
        img.put_pixel(1, 2, Rgba([0, 0, 0, 100]));
        let il = png(&img);
        let alto = Mascara::desde_ilustracion(&il, 128, 4, 0).unwrap();
        assert_eq!(cuenta(&alto), 0);
        let bajo = Mascara::desde_ilustracion(&il, 50, 4, 0).unwrap();
        assert_eq!(cuenta(&bajo), 1);
        assert!(bajo.get(1, 2));
    }

    #[test]
    fn umbral_es_inclusivo_en_el_valor_exacto() {
        let mut img = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 0]));
        img.put_pixel(1, 1, Rgba([0, 0, 0, 128]));
        let m = Mascara::desde_ilustracion(&png(&img), 128, 4, 0).unwrap();
        assert!(m.get(1, 1));
    }

    #[test]
    fn fondo_liso_usa_la_mediana_del_borde_e_ignora_manchas() {
        let mut img = RgbaImage::from_pixel(10, 10, Rgba([255, 255, 255, 255]));
        img.put_pixel(0, 0, Rgba([0, 0, 0, 255]));
        let m = Mascara::desde_ilustracion(&png(&img), 128, 10, 0).unwrap();
        assert_eq!(cuenta(&m), 1);
        assert!(m.get(0, 0));
    }

    #[test]
    fn fondo_liso_mira_todo_el_borde() {
        // Arriba e izquierda blancos; derecha y abajo negros (mayoría del
        // borde): el fondo es negro y lo blanco es figura.
        let mut img = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 255]));
        for i in 0..9 {
            img.put_pixel(i, 0, Rgba([255, 255, 255, 255]));
            img.put_pixel(0, i, Rgba([255, 255, 255, 255]));
        }
        let m = Mascara::desde_ilustracion(&png(&img), 128, 10, 0).unwrap();
        assert!(m.get(0, 0) && m.get(5, 0) && m.get(0, 5));
        assert!(!m.get(5, 5) && !m.get(9, 9));
    }

    fn verifica_circulo(il: &Ilustracion) {
        let m = Mascara::desde_ilustracion(il, 128, 40, 0).unwrap();
        assert!(m.get(20, 20));
        assert!(!m.get(1, 1));
        assert!(!m.get(38, 20));
        let area = cuenta(&m) as f64;
        let esperado = std::f64::consts::PI * 12.0 * 12.0;
        assert!((area - esperado).abs() < esperado * 0.1, "area {area}");
    }

    #[test]
    fn fondo_liso_blanco_png_y_jpg() {
        let img = circulo_sobre(Rgba([255, 255, 255, 255]), 40, 12.0);
        verifica_circulo(&png(&img));
        let j = Ilustracion::desde_bytes(codificar(&img, ImageFormat::Jpeg), Formato::Jpg).unwrap();
        verifica_circulo(&j);
    }

    #[test]
    fn fondo_liso_celeste() {
        let img = circulo_sobre(Rgba([135, 206, 235, 255]), 40, 12.0);
        verifica_circulo(&png(&img));
    }

    #[test]
    fn fondo_liso_detecta_linea_fina_clara() {
        // Diferencia de 40 niveles (>= 32) con fondo blanco: es figura.
        let mut img = RgbaImage::from_pixel(10, 10, Rgba([255, 255, 255, 255]));
        img.put_pixel(5, 5, Rgba([215, 255, 255, 255]));
        img.put_pixel(6, 5, Rgba([240, 255, 255, 255]));
        let m = Mascara::desde_ilustracion(&png(&img), 128, 10, 0).unwrap();
        assert!(m.get(5, 5));
        assert!(!m.get(6, 5));
    }

    #[test]
    fn relleno_agranda_la_grilla_y_desplaza_la_figura() {
        let mut img = RgbaImage::from_pixel(6, 4, Rgba([0, 0, 0, 0]));
        img.put_pixel(0, 0, Rgba([0, 0, 0, 255]));
        let m = Mascara::desde_ilustracion(&png(&img), 128, 6, 3).unwrap();
        assert_eq!((m.ancho, m.alto), (12, 10));
        assert!(m.get(3, 3));
        assert_eq!(cuenta(&m), 1);
    }

    #[test]
    fn figura_principal_deja_el_blob_grande() {
        let m = mascara_de(&["###....#", "###....#", "###.....", "........"]);
        let f = m.figura_principal();
        assert_eq!(cuenta(&f), 9);
        assert!(f.get(0, 0));
        assert!(!f.get(7, 0));
    }

    #[test]
    fn figura_principal_rellena_agujero_pero_no_bahia() {
        let anillo = mascara_de(&["#####", "#...#", "#...#", "#####"]);
        assert_eq!(cuenta(&anillo.figura_principal()), 20);
        // Una C abierta hacia el borde no se rellena.
        let c = mascara_de(&["#####", "#....", "#....", "#####"]);
        assert_eq!(cuenta(&c.figura_principal()), 12);
        // Hueco que solo conecta con el fondo en diagonal: sigue siendo fondo.
        let diag = mascara_de(&["#####", "#.#..", "##.#.", ".###."]);
        assert!(!diag.figura_principal().get(2, 2));
    }

    #[test]
    fn figura_principal_no_une_diagonales() {
        let m = mascara_de(&["##..", "##..", "..##", "..##"]);
        let f = m.figura_principal();
        assert_eq!(cuenta(&f), 4);
        assert!(f.get(0, 0));
        assert!(!f.get(2, 2));
    }

    #[test]
    fn dilatar_pixel_aislado_radio_2_da_disco_de_13() {
        let mut m = Mascara::vacia(9, 9);
        m.set(4, 4, true);
        let d = m.dilatar(2.0);
        assert_eq!(cuenta(&d), 13);
        assert!(d.get(4, 2) && d.get(6, 4) && d.get(5, 5));
        assert!(!d.get(6, 6) && !d.get(4, 1));
    }

    #[test]
    fn dilatar_radio_cero_negativo_o_nan_es_identidad() {
        let m = mascara_de(&["..#.", "....", "#..."]);
        assert_eq!(m.dilatar(0.0), m);
        assert_eq!(m.dilatar(-3.0), m);
        assert_eq!(m.dilatar(f64::NAN), m);
    }

    #[test]
    fn dilatar_en_la_ultima_columna_no_salta_de_fila() {
        let mut m = Mascara::vacia(5, 3);
        m.set(4, 1, true);
        let d = m.dilatar(1.0);
        assert_eq!(cuenta(&d), 4);
        assert!(d.get(3, 1) && d.get(4, 0) && d.get(4, 2));
        assert!(!d.get(0, 2) && !d.get(0, 1));
    }

    #[test]
    fn dilatar_desde_contorno_iguala_a_fuerza_bruta() {
        let m = mascara_de(&[
            "............",
            "..######....",
            "..#....#....",
            "..#.##.#..#.",
            "..######....",
            "............",
        ]);
        for r in [1.0, 1.5, 2.0, 3.2] {
            let mut esperado = Mascara::vacia(m.ancho, m.alto);
            for y in 0..m.alto {
                for x in 0..m.ancho {
                    let cerca = (0..m.alto).any(|qy| {
                        (0..m.ancho).any(|qx| {
                            let d2 =
                                (x as f64 - qx as f64).powi(2) + (y as f64 - qy as f64).powi(2);
                            m.get(qx, qy) && d2 <= r * r
                        })
                    });
                    esperado.set(x, y, cerca);
                }
            }
            assert_eq!(m.dilatar(r), esperado, "radio {r}");
        }
    }
}
