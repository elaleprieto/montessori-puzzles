mod app;

use anyhow::{Context, Result, bail};
use clap::Parser;
use montessori_puzzles::Parametros;
use montessori_puzzles::mascara::{Formato, Ilustracion};
use montessori_puzzles::rompecabezas::{exportar_svg, generar};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    version,
    about = "Genera un rompecabezas Montessori en SVG a partir de una Ilustración"
)]
struct Args {
    /// Ilustración (PNG, JPG o SVG); sin ella se abre la app
    entrada: Option<PathBuf>,
    /// Cantidad de Piezas
    #[arg(long, default_value_t = Parametros::default().piezas as u32, value_parser = clap::value_parser!(u32).range(2..=12))]
    piezas: u32,
    /// Ancho de la Ilustración en mm
    #[arg(long, default_value_t = Parametros::default().ancho_mm)]
    ancho: f64,
    /// Umbral de figura (0-255)
    #[arg(long, default_value_t = Parametros::default().umbral)]
    umbral: u8,
    /// Margen entre el dibujo y la Silueta, en mm
    #[arg(long, default_value_t = Parametros::default().margen_mm)]
    margen: f64,
    /// Tamaño de la Pestaña (fracción)
    #[arg(long, default_value_t = Parametros::default().tamano_pestana)]
    pestana: f64,
    /// Irregularidad de la Pestaña
    #[arg(long, default_value_t = Parametros::default().jitter)]
    jitter: f64,
    /// Semilla de la Variante
    #[arg(long, default_value_t = Parametros::default().semilla)]
    semilla: u64,
    /// La Silueta es el borde de la Ilustración entera (ignora Umbral y Margen)
    #[arg(long)]
    rectangular: bool,
    /// Con --rectangular, redondea las esquinas con este radio, en mm
    #[arg(long, requires = "rectangular")]
    radio_esquinas: Option<f64>,
    /// Archivo SVG de salida
    #[arg(short = 'o', long)]
    salida: Option<PathBuf>,
}

/// Formato de la Ilustración según la extensión (sin importar mayúsculas) y
/// ruta de salida (la indicada o `<carpeta>/<nombre>-rompecabezas.svg`).
fn formato_y_salida(entrada: &Path, salida: Option<&Path>) -> Result<(Formato, PathBuf)> {
    let ext = entrada
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase);
    let formato = match ext.as_deref() {
        Some("png") => Formato::Png,
        Some("jpg" | "jpeg") => Formato::Jpg,
        Some("svg") => Formato::Svg,
        _ => bail!("Formato no soportado: la Ilustración debe ser .png, .jpg, .jpeg o .svg"),
    };
    let salida = match salida {
        Some(s) => s.to_path_buf(),
        None => {
            let nombre = entrada
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            entrada.with_file_name(format!("{nombre}-rompecabezas.svg"))
        }
    };
    Ok((formato, salida))
}

fn main() -> Result<()> {
    let args = Args::parse();
    let Some(entrada) = args.entrada.as_deref() else {
        return app::abrir();
    };
    let (formato, salida) = formato_y_salida(entrada, args.salida.as_deref())?;
    let bytes =
        std::fs::read(entrada).with_context(|| format!("No se pudo leer {}", entrada.display()))?;
    let ilustracion = Ilustracion::desde_bytes(bytes, formato)?;
    let parametros = Parametros {
        piezas: args.piezas as usize,
        ancho_mm: args.ancho,
        umbral: args.umbral,
        margen_mm: args.margen,
        tamano_pestana: args.pestana,
        jitter: args.jitter,
        semilla: args.semilla,
        silueta_rectangular: args.rectangular,
        esquinas_redondeadas: args.radio_esquinas.is_some(),
        radio_esquinas_mm: args
            .radio_esquinas
            .unwrap_or(Parametros::default().radio_esquinas_mm),
    };
    let r = generar(&ilustracion, &parametros)?;
    std::fs::write(&salida, exportar_svg(&r, &ilustracion))
        .with_context(|| format!("No se pudo escribir {}", salida.display()))?;
    eprintln!("Semilla usada: {}", r.semilla_usada);
    if let Some(aviso) = &r.aviso {
        eprintln!("Aviso: {aviso}");
    }
    eprintln!("Escrito {}", salida.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deduce_formato_sin_importar_mayusculas() {
        for (nombre, esperado) in [
            ("a.png", Formato::Png),
            ("a.PNG", Formato::Png),
            ("a.jpg", Formato::Jpg),
            ("a.JPeG", Formato::Jpg),
            ("a.Svg", Formato::Svg),
        ] {
            assert_eq!(
                formato_y_salida(Path::new(nombre), None).unwrap().0,
                esperado
            );
        }
    }

    #[test]
    fn otra_extension_o_ninguna_es_error() {
        assert!(formato_y_salida(Path::new("a.gif"), None).is_err());
        assert!(formato_y_salida(Path::new("a"), None).is_err());
    }

    #[test]
    fn salida_por_defecto_junto_a_la_entrada() {
        let (_, s) = formato_y_salida(Path::new("/x/y/Gato.PNG"), None).unwrap();
        assert_eq!(s, Path::new("/x/y/Gato-rompecabezas.svg"));
        let (_, s) = formato_y_salida(Path::new("gato.jpg"), None).unwrap();
        assert_eq!(s, Path::new("gato-rompecabezas.svg"));
    }

    #[test]
    fn los_argumentos_del_cli_se_leen() {
        let a = Args::try_parse_from(["x", "g.png", "--piezas", "7", "-o", "o.svg"]).unwrap();
        assert_eq!(
            (a.piezas, a.salida.as_deref()),
            (7, Some(Path::new("o.svg")))
        );
        assert!(Args::try_parse_from(["x", "g.png", "--piezas", "1"]).is_err());
        assert!(Args::try_parse_from(["x", "g.png", "--piezas", "13"]).is_err());
        let r =
            Args::try_parse_from(["x", "g.png", "--rectangular", "--radio-esquinas", "6"]).unwrap();
        assert_eq!((r.rectangular, r.radio_esquinas), (true, Some(6.0)));
        assert!(Args::try_parse_from(["x", "g.png", "--radio-esquinas", "6"]).is_err());
        let d = Args::try_parse_from(["x"]).unwrap();
        assert_eq!(d.piezas as usize, Parametros::default().piezas);
    }

    #[test]
    fn salida_explicita_gana() {
        let (_, s) = formato_y_salida(Path::new("a.png"), Some(Path::new("/tmp/o.svg"))).unwrap();
        assert_eq!(s, Path::new("/tmp/o.svg"));
    }
}
