use std::path::PathBuf;

use egui::{self, Align2, Color32, RichText, Ui};

/// Persistent state for the model manager panel.
pub struct ModelManager {
    /// Loaded model entries (one per glTF file, may contain multiple meshes).
    pub models: Vec<LoadedModel>,
    /// Path pending load (set by file picker, consumed by app loop).
    pub pending_load: Option<PathBuf>,
    /// Group IDs pending removal (consumed by app loop).
    pub pending_remove: Vec<u32>,
    /// True while a model is being parsed/uploaded to GPU.
    pub is_loading: bool,
    /// Name of the file currently being loaded (for display).
    pub loading_filename: String,
    /// Currently selected model's group ID, if any.
    pub selected_group_id: Option<u32>,
}

/// A model that was loaded at runtime.
pub struct LoadedModel {
    pub group_id: u32,
    pub filename: String,
    pub mesh_count: usize,
    pub position: [f32; 3],
    pub rotation_deg: [f32; 3],
    pub scale: [f32; 3],
}

impl ModelManager {
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            pending_load: None,
            pending_remove: Vec::new(),
            is_loading: false,
            loading_filename: String::new(),
            selected_group_id: None,
        }
    }

    /// Register a freshly loaded model so it appears in the UI.
    pub fn register_model(&mut self, group_id: u32, filename: &str, mesh_count: usize) {
        self.models.push(LoadedModel {
            group_id,
            filename: filename.to_string(),
            mesh_count,
            position: [0.0, 0.0, 0.0],
            rotation_deg: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        });
    }
}

/// Draw the model manager panel.
pub fn draw_model_panel(ctx: &egui::Context, manager: &mut ModelManager, panel_visible: bool) {
    // Always draw the welcome / loading overlays regardless of panel visibility
    draw_viewport_overlay(ctx, manager);

    if !panel_visible {
        return;
    }

    egui::Window::new(RichText::new("📦 Model Manager").strong())
        .id(egui::Id::new("model_panel"))
        .default_width(320.0)
        .min_width(280.0)
        .max_width(440.0)
        .default_pos([20.0, 20.0])
        .resizable(true)
        .collapsible(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(500.0)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;

                    // ── Upload button ──
                    draw_upload_section(ui, manager);
                    ui.separator();

                    // ── Loading indicator in panel ──
                    if manager.is_loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                RichText::new(format!("Loading {}...", manager.loading_filename))
                                    .color(Color32::from_rgb(255, 200, 80)),
                            );
                        });
                        ui.separator();
                    }

                    // ── Loaded models list ──
                    draw_loaded_models(ui, manager);

                    ui.add_space(12.0);
                });
        });
}

/// Draw centered viewport overlays: welcome screen or loading indicator.
fn draw_viewport_overlay(ctx: &egui::Context, manager: &ModelManager) {
    if manager.is_loading {
        // ── Loading overlay ──
        draw_loading_overlay(ctx, &manager.loading_filename);
    } else if manager.models.is_empty() {
        // ── Welcome / instruction overlay ──
        draw_welcome_overlay(ctx);
    }
}

/// Centered loading overlay with animated spinner.
fn draw_loading_overlay(ctx: &egui::Context, filename: &str) {
    let screen_rect = ctx.screen_rect();

    // Semi-transparent backdrop
    egui::Area::new(egui::Id::new("loading_backdrop"))
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            let painter = ui.painter();
            painter.rect_filled(
                screen_rect,
                0.0,
                Color32::from_rgba_premultiplied(0, 0, 0, 140),
            );
        });

    egui::Area::new(egui::Id::new("loading_overlay"))
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(Color32::from_rgba_premultiplied(15, 15, 25, 220))
                .rounding(12.0)
                .inner_margin(32.0)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.spinner();
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Loading Model...")
                                .size(20.0)
                                .color(Color32::from_rgb(200, 220, 255))
                                .strong(),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(filename)
                                .size(14.0)
                                .color(Color32::from_gray(140)),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Parsing geometry & textures...")
                                .size(12.0)
                                .color(Color32::from_gray(100))
                                .italics(),
                        );
                    });
                });
        });

    // Request repaint so spinner animates
    ctx.request_repaint();
}

/// Centered welcome overlay with usage instructions.
fn draw_welcome_overlay(ctx: &egui::Context) {
    egui::Area::new(egui::Id::new("welcome_overlay"))
        .anchor(Align2::CENTER_CENTER, [0.0, 20.0])
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(Color32::from_rgba_premultiplied(10, 10, 18, 200))
                .rounding(16.0)
                .inner_margin(40.0)
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        // Title
                        ui.label(
                            RichText::new("🔥 Rust 3D Render Engine")
                                .size(28.0)
                                .color(Color32::from_rgb(220, 180, 255))
                                .strong(),
                        );

                        ui.add_space(8.0);

                        ui.label(
                            RichText::new("Welcome! Load a 3D model to get started.")
                                .size(16.0)
                                .color(Color32::from_rgb(180, 190, 210)),
                        );

                        ui.add_space(20.0);

                        // Instructions frame
                        egui::Frame::none()
                            .fill(Color32::from_rgba_premultiplied(30, 30, 45, 180))
                            .rounding(8.0)
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.y = 8.0;

                                let hint = |ui: &mut Ui, icon: &str, key: &str, desc: &str| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(icon)
                                                .size(14.0),
                                        );
                                        ui.label(
                                            RichText::new(key)
                                                .size(13.0)
                                                .color(Color32::from_rgb(130, 200, 255))
                                                .strong(),
                                        );
                                        ui.label(
                                            RichText::new(desc)
                                                .size(13.0)
                                                .color(Color32::from_gray(160)),
                                        );
                                    });
                                };

                                hint(ui, "📂", "Upload GLB / glTF", "— Load a model from file");
                                hint(ui, "🖱", "Left Drag", "— Orbit camera");
                                hint(ui, "🖱", "Middle Drag", "— Pan camera");
                                hint(ui, "🖱", "Scroll Wheel", "— Zoom in / out");
                                hint(ui, "⌨", "WASD / QE", "— Fly camera");
                                hint(ui, "⌨", "Tab", "— Toggle panels");
                                hint(ui, "⌨", "Home", "— Reset camera");
                            });

                        ui.add_space(16.0);

                        ui.label(
                            RichText::new("Press Tab to open the Model Manager panel")
                                .size(13.0)
                                .color(Color32::from_gray(120))
                                .italics(),
                        );
                    });
                });
        });
}

fn draw_upload_section(ui: &mut Ui, manager: &mut ModelManager) {
    ui.horizontal(|ui| {
        let btn = ui.button(
            RichText::new("📂 Upload GLB / glTF")
                .color(Color32::from_rgb(120, 200, 255))
                .strong(),
        );
        if btn.clicked() && !manager.is_loading {
            // Open native file dialog (blocking — freezes frame during dialog)
            let file = rfd::FileDialog::new()
                .set_title("Select a glTF / GLB model")
                .add_filter("glTF models", &["glb", "gltf"])
                .pick_file();

            if let Some(path) = file {
                manager.loading_filename = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                manager.is_loading = true;
                manager.pending_load = Some(path);
            }
        }
    });

    ui.label(
        RichText::new("Supported: .glb, .gltf")
            .small()
            .color(Color32::from_gray(110)),
    );
}

fn draw_loaded_models(ui: &mut Ui, manager: &mut ModelManager) {
    if manager.models.is_empty() {
        ui.label(
            RichText::new("No models loaded yet.")
                .italics()
                .color(Color32::from_gray(100)),
        );
        return;
    }

    ui.label(
        RichText::new(format!("Loaded Models ({})", manager.models.len()))
            .strong(),
    );

    // Show Deselect button if a model is selected
    if let Some(selected_gid) = manager.selected_group_id {
        if let Some(selected_model) = manager.models.iter().find(|m| m.group_id == selected_gid) {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Selected: {}", selected_model.filename))
                        .color(Color32::from_rgb(255, 150, 40))
                        .strong(),
                );
                if ui.small_button("Deselect").clicked() {
                    manager.selected_group_id = None;
                }
            });
            ui.separator();
        }
    }

    let mut remove_idx: Option<usize> = None;

    for (i, model) in manager.models.iter_mut().enumerate() {
        let is_selected = manager.selected_group_id == Some(model.group_id);
        let mut header_text = RichText::new(format!(
            "{}  ({} mesh{})",
            model.filename,
            model.mesh_count,
            if model.mesh_count == 1 { "" } else { "es" }
        ));

        if is_selected {
            header_text = RichText::new(format!(
                "⭐ {}  ({} mesh{})",
                model.filename,
                model.mesh_count,
                if model.mesh_count == 1 { "" } else { "es" }
            ))
            .color(Color32::from_rgb(255, 150, 40))
            .strong();
        }

        let id = ui.make_persistent_id(format!("user_model_{}", model.group_id));
        let response = egui::CollapsingHeader::new(header_text)
            .id_salt(id)
            .default_open(true)
            .show(ui, |ui| {
                // Remove button
                ui.horizontal(|ui| {
                    if ui
                        .small_button(
                            RichText::new("🗑 Remove").color(Color32::from_rgb(220, 80, 80)),
                        )
                        .clicked()
                    {
                        remove_idx = Some(i);
                    }
                });

                // Position
                ui.label("Position");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(
                        egui::DragValue::new(&mut model.position[0])
                            .speed(0.05)
                            .range(-50.0..=50.0),
                    );
                    ui.label("Y");
                    ui.add(
                        egui::DragValue::new(&mut model.position[1])
                            .speed(0.05)
                            .range(-50.0..=50.0),
                    );
                    ui.label("Z");
                    ui.add(
                        egui::DragValue::new(&mut model.position[2])
                            .speed(0.05)
                            .range(-50.0..=50.0),
                    );
                });

                // Rotation
                ui.label("Rotation (deg)");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(
                        egui::DragValue::new(&mut model.rotation_deg[0])
                            .speed(0.5)
                            .range(-360.0..=360.0),
                    );
                    ui.label("Y");
                    ui.add(
                        egui::DragValue::new(&mut model.rotation_deg[1])
                            .speed(0.5)
                            .range(-360.0..=360.0),
                    );
                    ui.label("Z");
                    ui.add(
                        egui::DragValue::new(&mut model.rotation_deg[2])
                            .speed(0.5)
                            .range(-360.0..=360.0),
                    );
                });

                // Scale
                ui.label("Scale");
                ui.horizontal(|ui| {
                    ui.label("X");
                    ui.add(
                        egui::DragValue::new(&mut model.scale[0])
                            .speed(0.01)
                            .range(0.01..=50.0),
                    );
                    ui.label("Y");
                    ui.add(
                        egui::DragValue::new(&mut model.scale[1])
                            .speed(0.01)
                            .range(0.01..=50.0),
                    );
                    ui.label("Z");
                    ui.add(
                        egui::DragValue::new(&mut model.scale[2])
                            .speed(0.01)
                            .range(0.01..=50.0),
                    );
                });
            });

        if response.header_response.clicked() {
            manager.selected_group_id = Some(model.group_id);
        }
    }

    // Process removals
    if let Some(idx) = remove_idx {
        let group_id = manager.models[idx].group_id;
        manager.pending_remove.push(group_id);
        manager.models.remove(idx);
    }
}
