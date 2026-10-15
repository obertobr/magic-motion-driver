//! egui/eframe GUI control panel with live interactive remote feedback.

use std::sync::mpsc::Sender;
use eframe::egui::{self, Color32, Panel, Pos2, Rect, Stroke, Vec2};

use crate::domain::action::{Action, MediaKey, VirtualKey};
use crate::domain::button::Button;
use crate::domain::config::Config;
use crate::infra::autostart::{is_autostart_enabled, set_autostart};
use crate::infra::driver::{DriverCommand, DriverTelemetry};
use crate::presentation::hotspots::{find_hotspot_at, BUTTON_HOTSPOTS, REMOTE_IMG_HEIGHT, REMOTE_IMG_WIDTH};
use crate::presentation::tray::{SystemTray, TrayAction};

pub struct DriverApp {
    config: Config,
    driver_cmd_tx: Sender<DriverCommand>,
    latest_telemetry: DriverTelemetry,
    selected_button: Button,
    texture: Option<egui::TextureHandle>,
    tray: Option<SystemTray>,
    save_status: Option<(String, std::time::Instant)>,
    active_tab: Tab,
}

impl std::fmt::Debug for DriverApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriverApp")
            .field("config", &self.config)
            .field("selected_button", &self.selected_button)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Buttons,
    Motion,
    Sensors,
}

impl DriverApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        config: Config,
        driver_cmd_tx: Sender<DriverCommand>,
    ) -> Self {
        let tray = SystemTray::new().ok();

        Self {
            config,
            driver_cmd_tx,
            latest_telemetry: DriverTelemetry::default(),
            selected_button: Button::Ok,
            texture: None,
            tray,
            save_status: None,
            active_tab: Tab::Buttons,
        }
    }

    fn ensure_texture(&mut self, ctx: &egui::Context) {
        if self.texture.is_none() {
            let img_bytes = include_bytes!("../../assets/remote.png");
            if let Ok(dynamic_img) = image::load_from_memory(img_bytes) {
                let rgba = dynamic_img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                self.texture = Some(ctx.load_texture("remote_body", color_image, Default::default()));
            }
        }
    }

    pub fn update_telemetry(&mut self, telem: DriverTelemetry) {
        // Auto-select physically pressed button in UI
        for b in telem.active_buttons.iter() {
            if b.is_mappable() {
                self.selected_button = b;
            }
        }
        self.latest_telemetry = telem;
    }

    fn save_config(&mut self) {
        if let Err(e) = self.config.save() {
            self.save_status = Some((format!("Erro ao salvar: {e}"), std::time::Instant::now()));
        } else {
            let _ = self.driver_cmd_tx.send(DriverCommand::UpdateConfig(self.config.clone()));
            self.save_status = Some(("Configurações salvas!".to_string(), std::time::Instant::now()));
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.ensure_texture(&ctx);

        // Process System Tray events
        if let Some(ref tray) = self.tray {
            match tray.process_events(&self.driver_cmd_tx) {
                Some(TrayAction::OpenPanel) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                Some(TrayAction::Quit) => {
                    let _ = self.driver_cmd_tx.send(DriverCommand::Stop);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                None => {}
            }
        }

        // Intercept close button to hide to tray
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        // Top Header Panel
        Panel::top("header").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("LG Magic Motion");
                ui.label(egui::RichText::new("Driver & Painel").color(Color32::from_rgb(160, 160, 180)));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Lateral direita superior: bolinha de status (verde conectado / vermelha desconectado)
                    let dot_color = if self.latest_telemetry.connected {
                        Color32::from_rgb(40, 220, 100)
                    } else {
                        Color32::from_rgb(230, 60, 60)
                    };
                    let (dot_rect, dot_response) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::hover());
                    ui.painter().circle_filled(dot_rect.center(), 5.5, dot_color);
                    ui.painter().circle_stroke(
                        dot_rect.center(),
                        5.5,
                        Stroke::new(1.0, if self.latest_telemetry.connected { Color32::from_rgb(20, 160, 60) } else { Color32::from_rgb(160, 30, 30) }),
                    );

                    let tooltip = if self.latest_telemetry.connected {
                        format!("Conectado ({}) - {:.0} Hz", self.latest_telemetry.port_name, self.latest_telemetry.rate_hz)
                    } else {
                        "Desconectado (Aguardando receptor CP2102...)".to_string()
                    };
                    dot_response.on_hover_text(tooltip);

                    ui.add_space(10.0);

                    if ui.button("Recalibrar Centro").clicked() {
                        let _ = self.driver_cmd_tx.send(DriverCommand::Recenter);
                    }

                    let (cursor_txt, cursor_color) = if self.latest_telemetry.cursor_enabled {
                        ("Mira: ATIVA", Color32::from_rgb(40, 200, 100))
                    } else {
                        ("Mira: PAUSADA", Color32::from_rgb(220, 100, 80))
                    };
                    if ui.button(egui::RichText::new(cursor_txt).color(cursor_color).strong()).clicked() {
                        let _ = self.driver_cmd_tx.send(DriverCommand::ToggleCursor);
                    }
                });
            });
            ui.add_space(6.0);
        });

        // Bottom Footer Panel
        Panel::bottom("footer").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Salvar Configurações").clicked() {
                    self.save_config();
                }

                if let Some((ref msg, time)) = self.save_status {
                    if time.elapsed() < std::time::Duration::from_secs(4) {
                        ui.label(egui::RichText::new(msg).color(Color32::from_rgb(100, 220, 140)));
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("AN-MR200 / AN-MR100").small().color(Color32::DARK_GRAY));
                });
            });
            ui.add_space(4.0);
        });

        // Central Area
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                // Left: Remote Image with interactive hotspots
                ui.vertical(|ui| {
                    ui.set_width(200.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("Controle Físico").strong());
                        ui.label(egui::RichText::new("Passe o mouse ou clique no botão").small().color(Color32::GRAY));
                        ui.add_space(6.0);

                        if let Some(ref tex) = self.texture {
                            let display_h = 560.0f32;
                            let display_w = display_h * (REMOTE_IMG_WIDTH / REMOTE_IMG_HEIGHT);
                            let avail_w = ui.available_width();
                            let offset_x = ((avail_w - display_w) / 2.0).max(0.0);

                            let (full_rect, resp) = ui.allocate_exact_size(
                                Vec2::new(avail_w, display_h),
                                egui::Sense::click_and_drag(),
                            );

                            let img_rect = Rect::from_min_size(
                                Pos2::new(full_rect.min.x + offset_x, full_rect.min.y),
                                Vec2::new(display_w, display_h),
                            );

                            let painter = ui.painter_at(full_rect);
                            painter.image(
                                tex.id(),
                                img_rect,
                                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );

                            let scale_x = display_w / REMOTE_IMG_WIDTH;
                            let scale_y = display_h / REMOTE_IMG_HEIGHT;

                            let hover_pos = resp.hover_pos();
                            let hovered_button = hover_pos.and_then(|pos| {
                                if img_rect.contains(pos) {
                                    let norm_x = (pos.x - img_rect.min.x) / display_w;
                                    let norm_y = (pos.y - img_rect.min.y) / display_h;
                                    find_hotspot_at(norm_x, norm_y)
                                } else {
                                    None
                                }
                            });

                            if hovered_button.is_some() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            if resp.clicked() {
                                if let Some(pos) = resp.interact_pointer_pos() {
                                    if img_rect.contains(pos) {
                                        let norm_x = (pos.x - img_rect.min.x) / display_w;
                                        let norm_y = (pos.y - img_rect.min.y) / display_h;
                                        if let Some(btn) = find_hotspot_at(norm_x, norm_y) {
                                            if btn.is_mappable() {
                                                self.selected_button = btn;
                                                self.active_tab = Tab::Buttons;
                                            }
                                        }
                                    }
                                }
                            }

                            for spot in &BUTTON_HOTSPOTS {
                                let center = Pos2::new(
                                    img_rect.min.x + spot.cx * scale_x,
                                    img_rect.min.y + spot.cy * scale_y,
                                );
                                let radius = spot.radius * scale_x;

                                let is_active = self.latest_telemetry.active_buttons.contains(spot.button);
                                let is_selected = self.selected_button == spot.button;
                                let is_hovered = hovered_button == Some(spot.button);

                                if is_active {
                                    // Live animated pulse when pressed physically on the remote!
                                    painter.circle_filled(
                                        center,
                                        radius + 6.0,
                                        Color32::from_rgba_unmultiplied(0, 240, 180, 160),
                                    );
                                    painter.circle_stroke(
                                        center,
                                        radius + 8.0,
                                        Stroke::new(2.5, Color32::from_rgb(0, 255, 200)),
                                    );
                                } else if is_hovered {
                                    // Contorno amarelo ao passar o mouse em cima do botão
                                    painter.circle_filled(
                                        center,
                                        radius + 4.0,
                                        Color32::from_rgba_unmultiplied(250, 215, 20, 50),
                                    );
                                    painter.circle_stroke(
                                        center,
                                        radius + 4.0,
                                        Stroke::new(2.5, Color32::from_rgb(250, 215, 20)),
                                    );
                                } else if is_selected {
                                    // Destaque do botão selecionado atualmente para configuração
                                    painter.circle_filled(
                                        center,
                                        radius + 4.0,
                                        Color32::from_rgba_unmultiplied(60, 160, 255, 45),
                                    );
                                    painter.circle_stroke(
                                        center,
                                        radius + 4.0,
                                        Stroke::new(2.0, Color32::from_rgb(80, 180, 255)),
                                    );
                                }
                            }
                        }
                    });
                });

                ui.separator();

                // Right: Tabs
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.active_tab, Tab::Buttons, "Mapeamento de Botões");
                        ui.selectable_value(&mut self.active_tab, Tab::Motion, "Movimento & Mira");
                        ui.selectable_value(&mut self.active_tab, Tab::Sensors, "Sensores ao Vivo");
                    });
                    ui.separator();

                    match self.active_tab {
                        Tab::Buttons => self.render_buttons_tab(ui),
                        Tab::Motion => self.render_motion_tab(ui),
                        Tab::Sensors => self.render_sensors_tab(ui),
                    }
                });
            });
        });

        if self.latest_telemetry.connected || !self.latest_telemetry.active_buttons.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        }
    }
}

impl DriverApp {
    fn render_buttons_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configurar Ação do Botão");
        ui.add_space(4.0);

        let current_action = self.config.get_action(self.selected_button);

        ui.horizontal(|ui| {
            ui.label("Botão Selecionado:");
            egui::ComboBox::from_id_salt("btn_selector")
                .selected_text(format!("{:?}", self.selected_button))
                .show_ui(ui, |ui| {
                    for b in Button::mappable() {
                        ui.selectable_value(&mut self.selected_button, b, format!("{b:?}"));
                    }
                });
        });

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Ação atribuída para este botão no Windows:").strong());

        let mut new_action = current_action;

        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            ui.group(|ui| {
                ui.label(egui::RichText::new("Ações do Mouse:").strong());
                ui.selectable_value(&mut new_action, Action::MouseLeft, "Clique Esquerdo");
                ui.selectable_value(&mut new_action, Action::MouseRight, "Clique Direito");
                ui.selectable_value(&mut new_action, Action::MouseMiddle, "Clique do Meio");
                ui.selectable_value(&mut new_action, Action::MouseWheelUp, "Rolar Roda para Cima");
                ui.selectable_value(&mut new_action, Action::MouseWheelDown, "Rolar Roda para Baixo");
            });

            ui.add_space(4.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Controles de Mira / Sistema:").strong());
                ui.selectable_value(&mut new_action, Action::ToggleCursor, "Ativar / Pausar Mira");
                ui.selectable_value(&mut new_action, Action::Recenter, "Recalibrar Centro");
                ui.selectable_value(&mut new_action, Action::None, "Nenhuma Ação (Desativar)");
            });

            ui.add_space(4.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Teclas de Mídia:").strong());
                for mk in MediaKey::ALL {
                    ui.selectable_value(&mut new_action, Action::Media(mk), mk.label());
                }
            });

            ui.add_space(4.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Teclas do Teclado:").strong());
                for vk in VirtualKey::ALL {
                    ui.selectable_value(&mut new_action, Action::Key(vk), vk.label());
                }
            });
        });

        if new_action != current_action {
            self.config.set_action(self.selected_button, new_action);
            self.save_config();
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Resumo de Mapeamentos:").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Restaurar Botões Padrão").clicked() {
                    self.config.button_mappings = Config::default().button_mappings;
                    self.save_config();
                }
            });
        });
        egui::Grid::new("mapping_summary").striped(true).show(ui, |ui| {
            ui.label("Botão");
            ui.label("Ação Atual");
            ui.end_row();

            for b in Button::mappable() {
                ui.label(format!("{b:?}"));
                ui.label(self.config.get_action(b).label());
                ui.end_row();
            }
        });
    }

    fn render_motion_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Ajustes de Movimento e Câmera");
        ui.label("Configurações otimizadas para navegação no desktop e mira em jogos.");
        ui.add_space(10.0);

        let mut changed = false;

        ui.group(|ui| {
            ui.label(egui::RichText::new("Sensibilidade:").strong());
            changed |= ui.add(egui::Slider::new(&mut self.config.motion.sensitivity, 0.05..=1.5).text("Sensibilidade Geral")).changed();
            changed |= ui.add(egui::Slider::new(&mut self.config.motion.sensitivity_x, 0.2..=2.5).text("Multiplicador X (Yaw)")).changed();
            changed |= ui.add(egui::Slider::new(&mut self.config.motion.sensitivity_y, 0.2..=2.5).text("Multiplicador Y (Pitch)")).changed();
        });

        ui.add_space(8.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Precisão & Filtros:").strong());
            changed |= ui.add(egui::Slider::new(&mut self.config.motion.deadzone, 0.0..=6.0).text("Zona Morta (ADC)").suffix(" pts")).changed();
            ui.label(egui::RichText::new("Elimina tremor e ruído quando a mão está parada.").small().color(Color32::GRAY));

            changed |= ui.add(egui::Slider::new(&mut self.config.motion.acceleration, 1.0..=1.8).text("Curva de Aceleração")).changed();
            ui.label(egui::RichText::new("1.0 = Linear (1:1). Acima de 1.1 dá mais agilidade a movimentos rápidos.").small().color(Color32::GRAY));

            changed |= ui.add(egui::Slider::new(&mut self.config.motion.smoothing, 0.0..=0.75).text("Suavização (Low-Pass)")).changed();
            ui.label(egui::RichText::new("Filtro exponencial para movimentos sem serrilhado.").small().color(Color32::GRAY));
        });

        ui.add_space(8.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Opções do Sistema:").strong());
            changed |= ui.checkbox(&mut self.config.motion.invert_x, "Inverter Eixo X (Horizontal)").changed();
            changed |= ui.checkbox(&mut self.config.motion.invert_y, "Inverter Eixo Y (Vertical)").changed();

            let mut autostart = is_autostart_enabled();
            if ui.checkbox(&mut autostart, "Iniciar junto com o Windows").changed() {
                let _ = set_autostart(autostart);
                self.config.auto_start = autostart;
                self.save_config();
            }

            ui.add_space(6.0);
            if ui.button("Restaurar Padrões de Fábrica").clicked() {
                self.config = Config::default();
                self.save_config();
            }
        });

        ui.add_space(8.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Conexão e Porta COM:").strong());
            ui.label(
                egui::RichText::new("Por padrão, o driver detecta automaticamente a porta do receptor CP2102 da LG.")
                    .small()
                    .color(Color32::GRAY),
            );

            let mut available_ports = crate::infra::serial::list_available_ports();
            available_ports.sort();

            let current_port_label = match &self.config.port_override {
                Some(p) if !p.trim().is_empty() => format!("Manual: {p}"),
                _ => "Automático (Auto-detectar receptor CP2102)".to_string(),
            };

            let mut port_changed = false;
            egui::ComboBox::from_id_salt("port_override_selector")
                .selected_text(&current_port_label)
                .show_ui(ui, |ui| {
                    let is_auto = self.config.port_override.is_none();
                    if ui.selectable_label(is_auto, "Automático (Auto-detectar receptor CP2102)").clicked() && !is_auto {
                        self.config.port_override = None;
                        port_changed = true;
                    }

                    if !available_ports.is_empty() {
                        ui.separator();
                        for port in &available_ports {
                            let is_selected = self.config.port_override.as_deref() == Some(port.as_str());
                            if ui.selectable_label(is_selected, format!("Forçar {port}")).clicked() && !is_selected {
                                self.config.port_override = Some(port.clone());
                                port_changed = true;
                            }
                        }
                    }
                });

            if port_changed {
                self.save_config();
            }
        });

        if changed {
            let _ = self.driver_cmd_tx.send(DriverCommand::UpdateConfig(self.config.clone()));
        }
    }

    fn render_sensors_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Telemetria do Controle em Tempo Real");
        ui.add_space(8.0);

        let t = &self.latest_telemetry;

        ui.group(|ui| {
            ui.label(egui::RichText::new("Conexão & Frequência:").strong());
            let mode_str = match &self.config.port_override {
                Some(p) if !p.trim().is_empty() => format!("Manual ({p})"),
                _ => "Automático (Auto-detectar receptor CP2102)".to_string(),
            };
            ui.label(format!("Modo de Porta: {mode_str}"));
            ui.label(format!("Porta COM Ativa: {}", if t.port_name.is_empty() { "Nenhuma (Desconectado)" } else { &t.port_name }));
            ui.label(format!("Frequência: {:.1} Hz (~9 ms/pacote)", t.rate_hz));
            ui.label(format!("Mira Ativa: {}", if t.cursor_enabled { "Sim" } else { "Pausada" }));
        });

        ui.add_space(6.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Giroscópio (Movimento Angular):").strong());
            ui.label(format!("X (Yaw):   {:4} (Delta: {:+4})", t.gyro_x, t.gyro_dx));
            ui.label(format!("Y (Pitch): {:4} (Delta: {:+4})", t.gyro_y, t.gyro_dy));
            ui.label(format!("Centro Calibrado: X={:.1}, Y={:.1}", t.center_x, t.center_y));
        });

        ui.add_space(6.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Acelerômetro (Gravidade / Inclinação):").strong());
            ui.label(format!("Accel X: {:4}", t.accel_x));
            ui.label(format!("Accel Y: {:4}", t.accel_y));
            ui.label(format!("Accel Z: {:4}", t.accel_z));
        });

        ui.add_space(6.0);
        ui.group(|ui| {
            ui.label(egui::RichText::new("Botões Ativos:").strong());
            if t.active_buttons.is_empty() {
                ui.label(egui::RichText::new("Nenhum botão pressionado").color(Color32::GRAY));
            } else {
                for b in t.active_buttons.iter() {
                    ui.label(egui::RichText::new(format!("• {b:?} [Pressionado]")).color(Color32::from_rgb(0, 255, 180)).strong());
                }
            }
        });
    }
}
