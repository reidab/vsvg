use crate::types::transforms::Transforms;
use crate::types::{document::FlattenedDocument, Document, LayerID, PageSize};
use std::collections::HashMap;
use std::error::Error;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub(crate) struct Viewer {
    /// polylines derived from the document
    #[serde(skip)]
    document: FlattenedDocument,

    #[serde(skip)]
    page_size: Option<PageSize>,

    /// show points
    show_point: bool,

    /// show grid
    show_grid: bool,

    /// layer visibility
    #[serde(skip)]
    layer_visibility: HashMap<LayerID, bool>,
}

impl From<crate::types::Color> for egui::ecolor::Color32 {
    fn from(val: crate::types::Color) -> Self {
        egui::ecolor::Color32::from_rgba_unmultiplied(val.r, val.g, val.b, val.a)
    }
}

impl Viewer {
    /// Called once before the first frame.
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        document: FlattenedDocument,
        page_size: Option<PageSize>,
    ) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        /*
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }*/

        Viewer {
            document,
            page_size,
            show_point: false,
            show_grid: false,
            layer_visibility: HashMap::new(),
        }
    }
}

const SHADOW_OFFSET: f64 = 10.;

impl eframe::App for Viewer {
    /// Called by the framework to save state before shutdown.
    /*fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }*/

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                //////////////// file menu
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });

                //////////////// view menu
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_point, "Show points");
                    ui.checkbox(&mut self.show_grid, "Show grid");
                });

                //////////////// layer menu
                ui.menu_button("Layer", |ui| {
                    for lid in self.document.layers.keys() {
                        let visibility = self.layer_visibility.entry(*lid).or_insert(true);
                        ui.checkbox(visibility, format!("Layer {}", lid));
                    }
                });

                egui::warn_if_debug_build(ui);
            });
        });

        let panel_frame = egui::Frame::central_panel(&ctx.style())
            .inner_margin(egui::style::Margin::same(0.))
            .fill(egui::Color32::from_rgb(242, 242, 242));
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                let mut plot = egui::plot::Plot::new("svg_plot")
                    .data_aspect(1.0)
                    .show_background(false)
                    .auto_bounds_x()
                    .auto_bounds_y();

                if !self.show_grid {
                    plot = plot.x_grid_spacer(|_| vec![]).y_grid_spacer(|_| vec![]);
                }

                plot.show(ui, |plot_ui| {
                    // plot page size
                    if let Some(page_size) = self.page_size {
                        let page_frame = vec![
                            [0.0, 0.0],
                            [page_size.w, 0.0],
                            [page_size.w, -page_size.h],
                            [0.0, -page_size.h],
                        ];

                        // shadow
                        plot_ui.polygon(
                            egui::plot::Polygon::new(egui::plot::PlotPoints::from_iter(
                                page_frame
                                    .iter()
                                    .map(|p| [p[0] + SHADOW_OFFSET, p[1] - SHADOW_OFFSET]),
                            ))
                            .color(egui::Color32::from_rgb(180, 180, 180))
                            .fill_alpha(1.),
                        );

                        // background
                        plot_ui.polygon(
                            egui::plot::Polygon::new(egui::plot::PlotPoints::from_iter(
                                page_frame.iter().copied(),
                            ))
                            .color(egui::Color32::WHITE)
                            .fill_alpha(1.),
                        );

                        // frame
                        plot_ui.polygon(
                            egui::plot::Polygon::new(egui::plot::PlotPoints::from_iter(
                                page_frame.into_iter(),
                            ))
                            .color(egui::Color32::from_rgb(128, 128, 128))
                            .fill_alpha(0.0),
                        );
                    }

                    for (i, layer) in self.document.layers.iter() {
                        if !self.layer_visibility.get(&i).unwrap_or(&true) {
                            continue;
                        }

                        for path in layer.paths.iter() {
                            plot_ui.line(
                                egui::plot::Line::new(egui::plot::PlotPoints::from_iter(
                                    path.data.iter().copied(),
                                ))
                                .color(path.color)
                                .width(path.stroke_width as f32),
                            );

                            if self.show_point {
                                plot_ui.points(
                                    egui::plot::Points::new(egui::plot::PlotPoints::from_iter(
                                        path.data.iter().copied(),
                                    ))
                                    .color(path.color)
                                    .radius(path.stroke_width as f32 * 2.0),
                                );
                            }
                        }
                    }
                });
            });
    }
}

impl Document {
    pub fn show(&self, tolerance: f64) -> Result<(), Box<dyn Error>> {
        let native_options = eframe::NativeOptions::default();
        let page_size = self.page_size;
        let polylines = self.flatten(tolerance).scale_non_uniform(1.0, -1.0);

        eframe::run_native(
            "vsvg",
            native_options,
            Box::new(move |cc| {
                let style = egui::Style {
                    visuals: egui::Visuals::light(),
                    ..egui::Style::default()
                };
                cc.egui_ctx.set_style(style);
                Box::new(Viewer::new(cc, polylines, page_size))
            }),
        )?;

        Ok(())
    }
}
