//! Ronda 07: la app de escritorio. Carga una Ilustración, ajusta los
//! Parámetros con vista previa y exporta el SVG.

use crate::formato_y_salida;
use anyhow::{Context as _, Result};
use eframe::egui;
use montessori_puzzles::Parametros;
use montessori_puzzles::geometria::Rect;
use montessori_puzzles::mascara::{Ilustracion, rasterizar};
use montessori_puzzles::rompecabezas::{Rompecabezas, exportar_svg, generar};
use montessori_puzzles::vista::{aplanar, encuadrar, imagen_de_piezas};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// Lado mayor de la textura de la Ilustración en la vista previa.
const LADO_TEXTURA: usize = 1600;
/// Opacidad del color de las Piezas sobre la Ilustración.
const ALFA_PIEZAS: u8 = 90;
const ROJO_DE_CORTE: egui::Color32 = egui::Color32::from_rgb(255, 0, 0);
/// Ancho inicial del panel de Parámetros y entre qué anchos se puede arrastrar.
const ANCHO_PANEL: f32 = 320.0;
const ANCHOS_PANEL: std::ops::RangeInclusive<f32> = 260.0..=560.0;
/// Alto de los controles.
const ALTO_CONTROL: f32 = 26.0;

/// Controles más grandes que los de egui por defecto, para usarlos cómodos.
fn agrandar_controles(estilo: &mut egui::Style) {
    use egui::{FontFamily::Proportional, FontId, TextStyle};
    estilo.text_styles = [
        (TextStyle::Heading, FontId::new(20.0, Proportional)),
        (TextStyle::Body, FontId::new(15.0, Proportional)),
        (TextStyle::Button, FontId::new(15.0, Proportional)),
        (TextStyle::Small, FontId::new(11.5, Proportional)),
        (TextStyle::Monospace, FontId::monospace(13.5)),
    ]
    .into();
    let e = &mut estilo.spacing;
    e.interact_size = egui::vec2(48.0, ALTO_CONTROL);
    e.button_padding = egui::vec2(8.0, 4.0);
    e.item_spacing = egui::vec2(8.0, 6.0);
}

pub fn abrir() -> Result<()> {
    let opciones = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Rompecabezas Montessori")
            .with_inner_size([1100.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(
        "montessori-puzzles",
        opciones,
        Box::new(|cc| {
            cc.egui_ctx.all_styles_mut(agrandar_controles);
            Ok(Box::new(App::new()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("No se pudo abrir la ventana: {e}"))
}

/// La Ilustración abierta y su textura para la vista previa.
struct Cargada {
    ruta: PathBuf,
    ilustracion: Arc<Ilustracion>,
    textura: egui::TextureHandle,
}

/// El último Rompecabezas generado, listo para dibujar.
struct Vista {
    rompecabezas: Rompecabezas,
    piezas: egui::TextureHandle,
    /// Silueta y Uniones aplanadas, en mm.
    cortes: Vec<Vec<egui::Pos2>>,
}

/// Lo que devuelve el hilo de generación: el pedido al que responde y el resultado.
type Respuesta = (u64, Result<Rompecabezas>);

/// Un archivo elegido en un diálogo.
enum Elegido {
    Ilustracion(PathBuf),
    Salida(PathBuf),
}

struct App {
    parametros: Parametros,
    cargada: Option<Cargada>,
    vista: Option<Vista>,
    error: Option<String>,
    mensaje: Option<String>,
    /// Número del último pedido; las respuestas de pedidos anteriores se descartan.
    pedido: u64,
    /// Parámetros del último pedido, para saber si hay que generar de nuevo.
    pedidos_con: Option<Parametros>,
    generando: bool,
    tx: Sender<Respuesta>,
    rx: Receiver<Respuesta>,
    /// Hay un diálogo de archivo abierto; su elección llega por `rx_dialogo`.
    dialogo_abierto: bool,
    tx_dialogo: Sender<Option<Elegido>>,
    rx_dialogo: Receiver<Option<Elegido>>,
}

impl App {
    fn new() -> Self {
        let (tx, rx) = channel();
        let (tx_dialogo, rx_dialogo) = channel();
        Self {
            parametros: Parametros::default(),
            cargada: None,
            vista: None,
            error: None,
            mensaje: None,
            pedido: 0,
            pedidos_con: None,
            generando: false,
            tx,
            rx,
            dialogo_abierto: false,
            tx_dialogo,
            rx_dialogo,
        }
    }

    /// Abre un diálogo de archivo sin bloquear la UI. En macOS, un diálogo
    /// modal abierto desde `ui()` hace entrar en pánico a winit ("tried to
    /// handle event while another event is currently being handled"); desde
    /// otro hilo, rfd lo lanza en la cola principal, fuera del manejo de eventos.
    fn abrir_dialogo(
        &mut self,
        ctx: &egui::Context,
        dialogo: impl FnOnce() -> Option<Elegido> + Send + 'static,
    ) {
        if self.dialogo_abierto {
            return;
        }
        self.dialogo_abierto = true;
        let (tx, ctx) = (self.tx_dialogo.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(dialogo());
            ctx.request_repaint();
        });
    }

    fn recibir_dialogo(&mut self, ctx: &egui::Context) {
        while let Ok(elegido) = self.rx_dialogo.try_recv() {
            self.dialogo_abierto = false;
            match elegido {
                Some(Elegido::Ilustracion(ruta)) => self.abrir_ilustracion(ctx, &ruta),
                Some(Elegido::Salida(ruta)) => self.escribir_svg(&ruta),
                None => {}
            }
        }
    }

    fn abrir_ilustracion(&mut self, ctx: &egui::Context, ruta: &Path) {
        match cargar(ctx, ruta) {
            Ok(c) => {
                self.cargada = Some(c);
                self.vista = None;
                self.error = None;
                self.mensaje = None;
                self.pedidos_con = None;
                // Lo que esté generándose es de la Ilustración anterior.
                self.pedido += 1;
                self.generando = false;
            }
            Err(e) => self.error = Some(format!("{e:#}")),
        }
    }

    /// Recibe resultados y, si los Parámetros cambiaron, larga una generación.
    fn sincronizar(&mut self, ctx: &egui::Context) {
        while let Ok((pedido, resultado)) = self.rx.try_recv() {
            if pedido != self.pedido {
                continue;
            }
            self.generando = false;
            match resultado {
                Ok(r) => {
                    self.vista = Some(preparar_vista(ctx, r));
                    self.error = None;
                }
                Err(e) => {
                    self.vista = None;
                    self.error = Some(format!("{e:#}"));
                }
            }
        }
        let Some(cargada) = &self.cargada else {
            return;
        };
        if self.generando || self.pedidos_con == Some(self.parametros) {
            return;
        }
        self.pedido += 1;
        self.pedidos_con = Some(self.parametros);
        self.generando = true;
        let (pedido, parametros, ilustracion) =
            (self.pedido, self.parametros, cargada.ilustracion.clone());
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send((pedido, generar(&ilustracion, &parametros)));
            ctx.request_repaint();
        });
    }

    fn pedir_salida(&mut self, ctx: &egui::Context) {
        let Some(cargada) = &self.cargada else {
            return;
        };
        let sugerida = formato_y_salida(&cargada.ruta, None).map(|(_, s)| s).ok();
        self.abrir_dialogo(ctx, move || {
            let mut dialogo = rfd::FileDialog::new().add_filter("SVG", &["svg"]);
            if let Some(s) = &sugerida {
                if let Some(carpeta) = s.parent().filter(|c| !c.as_os_str().is_empty()) {
                    dialogo = dialogo.set_directory(carpeta);
                }
                if let Some(nombre) = s.file_name() {
                    dialogo = dialogo.set_file_name(nombre.to_string_lossy());
                }
            }
            dialogo.save_file().map(Elegido::Salida)
        });
    }

    fn escribir_svg(&mut self, salida: &Path) {
        let (Some(cargada), Some(vista)) = (&self.cargada, &self.vista) else {
            return;
        };
        let svg = exportar_svg(&vista.rompecabezas, &cargada.ilustracion);
        match std::fs::write(salida, svg) {
            Ok(()) => {
                self.mensaje = Some(format!("Escrito {}", salida.display()));
                self.error = None;
            }
            Err(e) => self.error = Some(format!("No se pudo escribir {}: {e}", salida.display())),
        }
    }

    fn panel_de_parametros(&mut self, ui: &mut egui::Ui) {
        ui.heading("Rompecabezas");
        ui.add_space(4.0);
        if ui
            .add_enabled(!self.dialogo_abierto, boton_ancho(ui, "Abrir Ilustración…"))
            .clicked()
        {
            self.abrir_dialogo(ui.ctx(), || {
                rfd::FileDialog::new()
                    .add_filter("Ilustración", &["png", "jpg", "jpeg", "svg"])
                    .pick_file()
                    .map(Elegido::Ilustracion)
            });
        }
        match &self.cargada {
            Some(c) => ui.label(nombre_de_archivo(&c.ruta)),
            None => ui.weak("O arrastrá un archivo a la ventana"),
        };
        ui.separator();

        let p = &mut self.parametros;
        let d = Parametros::default();
        deslizador(ui, "Piezas", &mut p.piezas, d.piezas, 2..=12, "");
        deslizador(
            ui,
            "Ancho",
            &mut p.ancho_mm,
            d.ancho_mm,
            40.0..=600.0,
            " mm",
        );
        ui.checkbox(&mut p.silueta_rectangular, "Silueta rectangular")
            .on_hover_text("Corta la Ilustración entera, sin buscar la figura");
        // Umbral y Margen solo cuentan para la Silueta de la figura.
        ui.add_enabled_ui(!p.silueta_rectangular, |ui| {
            deslizador(ui, "Umbral", &mut p.umbral, d.umbral, 0..=255, "");
            deslizador(
                ui,
                "Margen",
                &mut p.margen_mm,
                d.margen_mm,
                0.0..=10.0,
                " mm",
            );
        });
        ui.add_enabled_ui(p.silueta_rectangular, |ui| {
            ui.checkbox(&mut p.esquinas_redondeadas, "Esquinas redondeadas");
            ui.add_enabled_ui(p.esquinas_redondeadas, |ui| {
                deslizador(
                    ui,
                    "Radio de las esquinas",
                    &mut p.radio_esquinas_mm,
                    d.radio_esquinas_mm,
                    0.0..=50.0,
                    " mm",
                );
            });
        });
        deslizador(
            ui,
            "Pestaña",
            &mut p.tamano_pestana,
            d.tamano_pestana,
            0.05..=0.4,
            "",
        );
        deslizador(ui, "Irregularidad", &mut p.jitter, d.jitter, 0.0..=1.0, "");
        ui.label("Semilla");
        fila(ui, |ui| {
            restaurar(ui, &mut p.semilla, d.semilla);
            ui.add_sized(
                [ui.available_width(), ALTO_CONTROL],
                egui::DragValue::new(&mut p.semilla),
            );
        });
        let otra = self
            .vista
            .as_ref()
            .map(|v| v.rompecabezas.semilla_usada.wrapping_add(1));
        if ui
            .add_enabled(otra.is_some(), boton_ancho(ui, "Otra Variante"))
            .clicked()
            && let Some(s) = otra
        {
            self.parametros.semilla = s;
        }
        ui.separator();

        let listo = self.vista.is_some() && !self.generando && !self.dialogo_abierto;
        if ui
            .add_enabled(listo, boton_ancho(ui, "Exportar SVG…"))
            .clicked()
        {
            self.pedir_salida(ui.ctx());
        }
        if self.generando {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Generando…");
            });
        }
        if let Some(v) = &self.vista {
            let (w, h) = v.rompecabezas.lienzo_mm;
            ui.label(format!(
                "{w:.0} × {h:.0} mm · semilla usada {}",
                v.rompecabezas.semilla_usada
            ));
            if let Some(aviso) = &v.rompecabezas.aviso {
                ui.colored_label(ui.visuals().warn_fg_color, aviso);
            }
        }
        if let Some(e) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
        if let Some(m) = &self.mensaje {
            ui.label(m);
        }
    }

    fn vista_previa(&self, ui: &mut egui::Ui) {
        let (respuesta, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());
        let area = respuesta.rect.shrink(16.0);
        let Some(cargada) = &self.cargada else {
            painter.text(
                area.center(),
                egui::Align2::CENTER_CENTER,
                "Abrí una Ilustración (PNG, JPG o SVG)",
                egui::FontId::proportional(18.0),
                ui.visuals().weak_text_color(),
            );
            return;
        };
        let Some(v) = &self.vista else {
            // Antes del primer resultado, la Ilustración sola.
            let [w, h] = cargada.textura.size();
            let (r, _) = encuadrar((w as f64, h as f64), a_rect(area));
            painter.image(
                cargada.textura.id(),
                a_egui(r),
                uv_completa(),
                egui::Color32::WHITE,
            );
            return;
        };
        let rz = &v.rompecabezas;
        let (lienzo, escala) = encuadrar(rz.lienzo_mm, a_rect(area));
        let lienzo = a_egui(lienzo);
        let a_pantalla = |p: egui::Pos2| lienzo.min + p.to_vec2() * escala as f32;
        let il = rz.ilustracion_en;

        painter.rect_filled(lienzo, 0.0, egui::Color32::WHITE);
        painter.image(
            cargada.textura.id(),
            egui::Rect::from_min_max(
                a_pantalla(egui::pos2(il.x as f32, il.y as f32)),
                a_pantalla(egui::pos2(
                    (il.x + il.ancho) as f32,
                    (il.y + il.alto) as f32,
                )),
            ),
            uv_completa(),
            egui::Color32::WHITE,
        );
        painter.image(v.piezas.id(), lienzo, uv_completa(), egui::Color32::WHITE);
        let trazo = egui::Stroke::new(1.5, ROJO_DE_CORTE);
        for corte in &v.cortes {
            painter.line(corte.iter().map(|&p| a_pantalla(p)).collect(), trazo);
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let soltado = ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()));
        if let Some(ruta) = soltado {
            self.abrir_ilustracion(ctx, &ruta);
        }
        self.recibir_dialogo(ctx);
        self.sincronizar(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::left("parametros")
            .resizable(true)
            .default_size(ANCHO_PANEL)
            .size_range(ANCHOS_PANEL)
            .show(ui, |ui| self.panel_de_parametros(ui));
        egui::CentralPanel::default().show(ui, |ui| self.vista_previa(ui));
    }
}

fn cargar(ctx: &egui::Context, ruta: &Path) -> Result<Cargada> {
    let (formato, _) = formato_y_salida(ruta, None)?;
    let bytes =
        std::fs::read(ruta).with_context(|| format!("No se pudo leer {}", ruta.display()))?;
    let ilustracion = Ilustracion::desde_bytes(bytes, formato)?;
    let raster = rasterizar(&ilustracion, LADO_TEXTURA)?;
    let imagen = egui::ColorImage::from_rgba_unmultiplied(
        [raster.width() as usize, raster.height() as usize],
        raster.as_raw(),
    );
    Ok(Cargada {
        ruta: ruta.to_path_buf(),
        ilustracion: Arc::new(ilustracion),
        textura: ctx.load_texture("ilustracion", imagen, egui::TextureOptions::LINEAR),
    })
}

fn preparar_vista(ctx: &egui::Context, rompecabezas: Rompecabezas) -> Vista {
    let piezas = imagen_de_piezas(&rompecabezas.etiquetas, ALFA_PIEZAS);
    let imagen = egui::ColorImage::from_rgba_unmultiplied(
        [piezas.width() as usize, piezas.height() as usize],
        piezas.as_raw(),
    );
    let cortes = rompecabezas
        .silueta
        .iter()
        .chain(&rompecabezas.uniones)
        .map(|t| {
            aplanar(t)
                .into_iter()
                .map(|p| egui::pos2(p.x as f32, p.y as f32))
                .collect()
        })
        .collect();
    Vista {
        piezas: ctx.load_texture("piezas", imagen, egui::TextureOptions::NEAREST),
        cortes,
        rompecabezas,
    }
}

/// Un botón que ocupa todo el ancho del panel.
fn boton_ancho<'a>(ui: &egui::Ui, texto: &'a str) -> egui::Button<'a> {
    egui::Button::new(texto).min_size(egui::vec2(ui.available_width(), ALTO_CONTROL + 4.0))
}

/// Ancho del botón ↺ y de la caja con el valor de cada Parámetro.
const ANCHO_RESTAURAR: f32 = 30.0;
const ANCHO_VALOR: f32 = 76.0;

/// Una fila de controles armada de derecha a izquierda: lo primero que se
/// agrega queda pegado al borde derecho y lo último se lleva lo que sobra.
fn fila(ui: &mut egui::Ui, contenido: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), ALTO_CONTROL),
        egui::Layout::right_to_left(egui::Align::Center),
        contenido,
    );
}

/// Un Parámetro: su nombre arriba y, debajo, el botón ↺ y la caja del valor
/// en columnas fijas a la derecha y el deslizador en el resto del ancho.
fn deslizador<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    nombre: &str,
    valor: &mut T,
    defecto: T,
    rango: std::ops::RangeInclusive<T>,
    sufijo: &str,
) {
    ui.label(nombre);
    fila(ui, |ui| {
        restaurar(ui, valor, defecto);
        ui.add_sized(
            [ANCHO_VALOR, ALTO_CONTROL],
            egui::DragValue::new(valor)
                .range(rango.clone())
                .suffix(sufijo),
        );
        ui.spacing_mut().slider_width = ui.available_width();
        ui.add(egui::Slider::new(valor, rango).show_value(false));
    });
}

fn restaurar<T: PartialEq>(ui: &mut egui::Ui, valor: &mut T, defecto: T) {
    let boton = egui::Button::new("↺").min_size(egui::vec2(ANCHO_RESTAURAR, ALTO_CONTROL));
    if ui
        .add_enabled(*valor != defecto, boton)
        .on_hover_text("Volver al valor por defecto")
        .on_disabled_hover_text("Ya está en el valor por defecto")
        .clicked()
    {
        *valor = defecto;
    }
}

fn nombre_de_archivo(ruta: &Path) -> String {
    ruta.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn uv_completa() -> egui::Rect {
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
}

fn a_rect(r: egui::Rect) -> Rect {
    Rect {
        x: r.min.x as f64,
        y: r.min.y as f64,
        ancho: r.width() as f64,
        alto: r.height() as f64,
    }
}

fn a_egui(r: Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(r.x as f32, r.y as f32),
        egui::vec2(r.ancho as f32, r.alto as f32),
    )
}
