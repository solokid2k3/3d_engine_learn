use egui::{self, Color32, RichText, Ui};

use crate::gui::light_settings::{LightSettings, PointLightSettings};

/// Draw the light control panel. Returns `true` if the panel is visible.
pub fn draw_light_panel(ctx: &egui::Context, settings: &mut LightSettings) {
    if !settings.panel_visible {
        return;
    }

    egui::Window::new(RichText::new("🔆 Light System").strong())
        .id(egui::Id::new("light_panel"))
        .default_width(300.0)
        .min_width(260.0)
        .max_width(420.0)
        .default_pos([920.0, 20.0])
        .resizable(true)
        .collapsible(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(640.0)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;

                    // ── Header info ──
                    ui.label(
                        RichText::new("Press Tab to toggle this panel")
                            .small()
                            .color(Color32::from_gray(120)),
                    );
                    ui.separator();

                    // ── Directional Light ──
                    draw_directional_section(ui, settings);
                    ui.separator();

                    // ── Point Lights ──
                    draw_point_lights_section(ui, settings);
                    ui.separator();

                    // ── Environment ──
                    draw_environment_section(ui, settings);

                    ui.add_space(20.0);
                });
        });
}

fn draw_directional_section(ui: &mut Ui, settings: &mut LightSettings) {
    let dir = &mut settings.directional;

    ui.horizontal(|ui| {
        ui.checkbox(&mut dir.enabled, "");
        ui.label(RichText::new("☀ Directional Light").strong());
    });

    if !dir.enabled {
        return;
    }

    ui.indent("dir_indent", |ui| {
        ui.label("Direction");
        ui.horizontal(|ui| {
            ui.label("X");
            ui.add(egui::DragValue::new(&mut dir.direction[0]).speed(0.01).range(-1.0..=1.0));
            ui.label("Y");
            ui.add(egui::DragValue::new(&mut dir.direction[1]).speed(0.01).range(-1.0..=1.0));
            ui.label("Z");
            ui.add(egui::DragValue::new(&mut dir.direction[2]).speed(0.01).range(-1.0..=1.0));
        });

        ui.horizontal(|ui| {
            ui.label("Color");
            let mut color = dir.color;
            if ui.color_edit_button_rgb(&mut color).changed() {
                dir.color = color;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Intensity");
            ui.add(egui::Slider::new(&mut dir.intensity, 0.0..=5.0).step_by(0.01));
        });
    });
}

fn draw_point_lights_section(ui: &mut Ui, settings: &mut LightSettings) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("💡 Point Lights").strong());
        ui.label(
            RichText::new(format!("{}/4", settings.point_lights.len()))
                .small()
                .color(Color32::from_gray(140)),
        );
    });

    let mut remove_index: Option<usize> = None;

    for (i, light) in settings.point_lights.iter_mut().enumerate() {
        let id = ui.make_persistent_id(format!("point_light_{}", i));
        egui::CollapsingHeader::new(
            RichText::new(format!("{}  {}", if light.enabled { "●" } else { "○" }, light.label)),
        )
        .id_salt(id)
        .default_open(i == 0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut light.enabled, "Enabled");
                ui.checkbox(&mut light.animated, "Animated");
                if ui
                    .small_button(RichText::new("🗑").color(Color32::from_rgb(220, 80, 80)))
                    .clicked()
                {
                    remove_index = Some(i);
                }
            });

            if !light.enabled {
                return;
            }

            // Label
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut light.label);
            });

            // Position
            ui.label("Position");
            ui.horizontal(|ui| {
                ui.label("X");
                ui.add(egui::DragValue::new(&mut light.position[0]).speed(0.05).range(-20.0..=20.0));
                ui.label("Y");
                ui.add(egui::DragValue::new(&mut light.position[1]).speed(0.05).range(-20.0..=20.0));
                ui.label("Z");
                ui.add(egui::DragValue::new(&mut light.position[2]).speed(0.05).range(-20.0..=20.0));
            });

            // Color
            ui.horizontal(|ui| {
                ui.label("Color");
                let mut color = light.color;
                if ui.color_edit_button_rgb(&mut color).changed() {
                    light.color = color;
                }
            });

            // Intensity
            ui.horizontal(|ui| {
                ui.label("Intensity");
                ui.add(egui::Slider::new(&mut light.intensity, 0.0..=10.0).step_by(0.01));
            });

            // Attenuation (collapsible)
            egui::CollapsingHeader::new("Attenuation")
                .id_salt(format!("atten_{}", i))
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Constant");
                        ui.add(
                            egui::DragValue::new(&mut light.constant)
                                .speed(0.01)
                                .range(0.0..=5.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Linear");
                        ui.add(
                            egui::DragValue::new(&mut light.linear)
                                .speed(0.001)
                                .range(0.0..=1.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Quadratic");
                        ui.add(
                            egui::DragValue::new(&mut light.quadratic)
                                .speed(0.001)
                                .range(0.0..=1.0),
                        );
                    });
                });
        });
    }

    // Remove light
    if let Some(idx) = remove_index {
        settings.point_lights.remove(idx);
    }

    // Add light button
    if settings.point_lights.len() < 4 {
        if ui
            .button(RichText::new("＋ Add Point Light").color(Color32::from_rgb(100, 200, 130)))
            .clicked()
        {
            let n = settings.point_lights.len();
            settings.point_lights.push(PointLightSettings::new(
                &format!("Light {}", n + 1),
                [0.0, 2.0, 0.0],
                [1.0, 1.0, 1.0],
                1.5,
            ));
            // New lights default to non-animated
            settings.point_lights.last_mut().unwrap().animated = false;
        }
    }
}

fn draw_environment_section(ui: &mut Ui, settings: &mut LightSettings) {
    let env = &mut settings.environment;

    ui.label(RichText::new("🌫 Environment").strong());

    ui.indent("env_indent", |ui| {
        // Fog
        ui.label("Fog");
        ui.horizontal(|ui| {
            ui.label("Density");
            ui.add(egui::Slider::new(&mut env.fog_density, 0.0..=0.2).step_by(0.001));
        });
        ui.horizontal(|ui| {
            ui.label("Color");
            let mut color = env.fog_color;
            if ui.color_edit_button_rgb(&mut color).changed() {
                env.fog_color = color;
            }
        });

        ui.add_space(4.0);

        // Rim
        ui.label("Rim Light");
        ui.horizontal(|ui| {
            ui.label("Strength");
            ui.add(egui::Slider::new(&mut env.rim_strength, 0.0..=2.0).step_by(0.01));
        });
        ui.horizontal(|ui| {
            ui.label("Power");
            ui.add(egui::Slider::new(&mut env.rim_power, 1.0..=8.0).step_by(0.1));
        });
        ui.horizontal(|ui| {
            ui.label("Color");
            let mut color = env.rim_color;
            if ui.color_edit_button_rgb(&mut color).changed() {
                env.rim_color = color;
            }
        });
    });
}
