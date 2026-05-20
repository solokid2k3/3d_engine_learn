use egui::{self, Color32, RichText};
use glam::Vec3;
use crate::scene::particle::{ParticleSystem, ParticleEmitterShape, ParticleBlendMode, ParticleRenderType};

pub fn draw_vfx_panel(
    ctx: &egui::Context,
    vfx: &mut ParticleSystem,
    emitter_selected: &mut bool,
    panel_visible: bool,
) {
    if !panel_visible {
        return;
    }

    egui::Window::new(RichText::new("✨ VFX / Particle Editor").strong())
        .id(egui::Id::new("vfx_panel"))
        .default_width(320.0)
        .min_width(280.0)
        .max_width(440.0)
        .default_pos([20.0, 300.0])
        .resizable(true)
        .collapsible(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(600.0)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;

                    // ── Presets Section ──
                    ui.label(RichText::new("🎯 Presets").strong());
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(RichText::new("🔥 Fire").color(Color32::from_rgb(255, 120, 50))).clicked() {
                            *vfx = ParticleSystem::fire();
                        }
                        if ui.button(RichText::new("✨ Magic").color(Color32::from_rgb(180, 100, 255))).clicked() {
                            *vfx = ParticleSystem::magic();
                        }
                        if ui.button(RichText::new("💨 Smoke").color(Color32::from_rgb(180, 180, 180))).clicked() {
                            *vfx = ParticleSystem::smoke();
                        }
                        if ui.button(RichText::new("💥 Explosion").color(Color32::from_rgb(255, 80, 80))).clicked() {
                            *vfx = ParticleSystem::explosion();
                        }
                        if ui.button(RichText::new("🌧 Rain").color(Color32::from_rgb(80, 150, 255))).clicked() {
                            *vfx = ParticleSystem::rain();
                        }
                    });
                    ui.separator();

                    // ── Simulation Controls ──
                    ui.label(RichText::new("🎮 Simulation").strong());
                    ui.horizontal(|ui| {
                        let play_label = if vfx.is_playing { "⏸ Pause" } else { "▶ Play" };
                        if ui.button(play_label).clicked() {
                            vfx.is_playing = !vfx.is_playing;
                        }
                        if ui.button("🔄 Reset").clicked() {
                            vfx.particles.clear();
                        }
                        ui.checkbox(emitter_selected, "🖱 Manipulate Emitter");
                    });
                    ui.label(
                        RichText::new(format!("Active Particles: {}", vfx.particles.len()))
                            .small()
                            .color(Color32::from_gray(140)),
                    );
                    ui.separator();

                    // ── Emitter Position & Shape ──
                    ui.label(RichText::new("📍 Emitter Settings").strong());
                    ui.indent("emitter_pos_indent", |ui| {
                        ui.label("Position");
                        ui.horizontal(|ui| {
                            ui.label("X");
                            ui.add(egui::DragValue::new(&mut vfx.position.x).speed(0.05));
                            ui.label("Y");
                            ui.add(egui::DragValue::new(&mut vfx.position.y).speed(0.05));
                            ui.label("Z");
                            ui.add(egui::DragValue::new(&mut vfx.position.z).speed(0.05));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Shape");
                            let mut current_shape = match vfx.settings.shape {
                                ParticleEmitterShape::Point => 0,
                                ParticleEmitterShape::Sphere { .. } => 1,
                                ParticleEmitterShape::Box { .. } => 2,
                            };
                            if egui::ComboBox::from_label("")
                                .selected_text(match current_shape {
                                    0 => "Point",
                                    1 => "Sphere",
                                    2 => "Box",
                                    _ => "",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut current_shape, 0, "Point");
                                    ui.selectable_value(&mut current_shape, 1, "Sphere");
                                    ui.selectable_value(&mut current_shape, 2, "Box");
                                })
                                .response
                                .changed()
                            {
                                vfx.settings.shape = match current_shape {
                                    0 => ParticleEmitterShape::Point,
                                    1 => ParticleEmitterShape::Sphere { radius: 1.0 },
                                    2 => ParticleEmitterShape::Box { extents: Vec3::new(1.0, 0.1, 1.0) },
                                    _ => ParticleEmitterShape::Point,
                                };
                            }
                        });

                        match &mut vfx.settings.shape {
                            ParticleEmitterShape::Sphere { radius } => {
                                ui.horizontal(|ui| {
                                    ui.label("Radius");
                                    ui.add(egui::Slider::new(radius, 0.05..=10.0));
                                });
                            }
                            ParticleEmitterShape::Box { extents } => {
                                ui.horizontal(|ui| {
                                    ui.label("Extents");
                                    ui.add(egui::DragValue::new(&mut extents.x).speed(0.02).range(0.0..=10.0));
                                    ui.add(egui::DragValue::new(&mut extents.y).speed(0.02).range(0.0..=10.0));
                                    ui.add(egui::DragValue::new(&mut extents.z).speed(0.02).range(0.0..=10.0));
                                });
                            }
                            _ => {}
                        }
                    });
                    ui.separator();

                    // ── Spawn & Lifetime Parameters ──
                    ui.label(RichText::new("⏳ Spawn & Lifetime").strong());
                    ui.indent("spawn_lifetime_indent", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Max Particles");
                            ui.add(egui::DragValue::new(&mut vfx.settings.max_particles).speed(5.0).range(1..=5000));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Spawn Rate (/s)");
                            ui.add(egui::Slider::new(&mut vfx.settings.spawn_rate, 1.0..=1000.0).logarithmic(true));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Min Lifetime (s)");
                            ui.add(egui::DragValue::new(&mut vfx.settings.min_lifetime).speed(0.05).range(0.05..=20.0));
                            ui.label("Max Lifetime (s)");
                            ui.add(egui::DragValue::new(&mut vfx.settings.max_lifetime).speed(0.05).range(0.05..=20.0));
                        });
                    });
                    ui.separator();

                    // ── Physics Parameters ──
                    ui.label(RichText::new("🚀 Physics").strong());
                    ui.indent("physics_indent", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Min Speed");
                            ui.add(egui::DragValue::new(&mut vfx.settings.min_speed).speed(0.05).range(0.0..=50.0));
                            ui.label("Max Speed");
                            ui.add(egui::DragValue::new(&mut vfx.settings.max_speed).speed(0.05).range(0.0..=50.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Start Size");
                            ui.add(egui::DragValue::new(&mut vfx.settings.start_size).speed(0.01).range(0.01..=5.0));
                            ui.label("End Size");
                            ui.add(egui::DragValue::new(&mut vfx.settings.end_size).speed(0.01).range(0.0..=5.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Gravity");
                            ui.add(egui::DragValue::new(&mut vfx.settings.gravity.x).speed(0.1));
                            ui.add(egui::DragValue::new(&mut vfx.settings.gravity.y).speed(0.1));
                            ui.add(egui::DragValue::new(&mut vfx.settings.gravity.z).speed(0.1));
                        });
                    });
                    ui.separator();

                    // ── Rendering Style & Blend Mode ──
                    ui.label(RichText::new("🎨 Visuals").strong());
                    ui.indent("visuals_indent", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Blend Mode");
                            let mut blend = match vfx.settings.blend_mode {
                                ParticleBlendMode::Additive => 0,
                                ParticleBlendMode::Alpha => 1,
                            };
                            if ui.selectable_value(&mut blend, 0, "Additive").changed() {
                                vfx.settings.blend_mode = ParticleBlendMode::Additive;
                            }
                            if ui.selectable_value(&mut blend, 1, "Alpha").changed() {
                                vfx.settings.blend_mode = ParticleBlendMode::Alpha;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Shape Style");
                            let mut style = match vfx.settings.render_type {
                                ParticleRenderType::GlowCircle => 0,
                                ParticleRenderType::Spark => 1,
                                ParticleRenderType::Flame => 2,
                                ParticleRenderType::Smoke => 3,
                                ParticleRenderType::Star => 4,
                            };
                            if egui::ComboBox::from_label("")
                                .selected_text(match style {
                                    0 => "Glow Circle",
                                    1 => "Spark/Flare",
                                    2 => "Flame",
                                    3 => "Smoke/Cloud",
                                    4 => "Star",
                                    _ => "",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut style, 0, "Glow Circle");
                                    ui.selectable_value(&mut style, 1, "Spark/Flare");
                                    ui.selectable_value(&mut style, 2, "Flame");
                                    ui.selectable_value(&mut style, 3, "Smoke/Cloud");
                                    ui.selectable_value(&mut style, 4, "Star");
                                })
                                .response
                                .changed()
                            {
                                vfx.settings.render_type = match style {
                                    0 => ParticleRenderType::GlowCircle,
                                    1 => ParticleRenderType::Spark,
                                    2 => ParticleRenderType::Flame,
                                    3 => ParticleRenderType::Smoke,
                                    4 => ParticleRenderType::Star,
                                    _ => ParticleRenderType::GlowCircle,
                                };
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Start Color");
                            let mut color = [
                                vfx.settings.start_color.x,
                                vfx.settings.start_color.y,
                                vfx.settings.start_color.z,
                                vfx.settings.start_color.w,
                            ];
                            if ui.color_edit_button_rgba_unmultiplied(&mut color).changed() {
                                vfx.settings.start_color = glam::Vec4::new(color[0], color[1], color[2], color[3]);
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("End Color");
                            let mut color = [
                                vfx.settings.end_color.x,
                                vfx.settings.end_color.y,
                                vfx.settings.end_color.z,
                                vfx.settings.end_color.w,
                            ];
                            if ui.color_edit_button_rgba_unmultiplied(&mut color).changed() {
                                vfx.settings.end_color = glam::Vec4::new(color[0], color[1], color[2], color[3]);
                            }
                        });
                    });

                    ui.add_space(10.0);
                });
        });
}
