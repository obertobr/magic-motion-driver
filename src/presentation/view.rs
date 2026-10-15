//! egui/eframe GUI control panel with live interactive remote feedback.

use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};
use eframe::egui::{self, Color32, Panel, Pos2, Rect, Stroke, StrokeKind, Vec2};

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
    telem_rx: Option<Receiver<DriverTelemetry>>,
    latest_telemetry: DriverTelemetry,
    selected_button: Button,
    texture: Option<egui::TextureHandle>,
    tray: Option<SystemTray>,
    save_status: Option<(String, std::time::Instant)>,
    calibration_feedback: Option<(String, std::time::Instant)>,
    active_tab: Tab,
    pub should_exit: bool,
    gyro_history_x: VecDeque<f32>,
    gyro_history_y: VecDeque<f32>,
    accel_history_x: VecDeque<f32>,
    accel_history_y: VecDeque<f32>,
    accel_history_z: VecDeque<f32>,
}

impl std::fmt::Debug for DriverApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriverApp")
            .field("config", &self.config)
            .field("selected_button", &self.selected_button)
            .field("should_exit", &self.should_exit)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Buttons,
    Motion,
    Settings,
    Sensors,
}

impl DriverApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        config: Config,
        driver_cmd_tx: Sender<DriverCommand>,
        telem_rx: Option<Receiver<DriverTelemetry>>,
    ) -> Self {
        let tray = SystemTray::new().ok();

        Self {
            config,
            driver_cmd_tx,
            telem_rx,
            latest_telemetry: DriverTelemetry::default(),
            selected_button: Button::Ok,
            texture: None,
            tray,
            save_status: None,
            calibration_feedback: None,
            active_tab: Tab::Buttons,
            should_exit: false,
            gyro_history_x: VecDeque::with_capacity(120),
            gyro_history_y: VecDeque::with_capacity(120),
            accel_history_x: VecDeque::with_capacity(120),
            accel_history_y: VecDeque::with_capacity(120),
            accel_history_z: VecDeque::with_capacity(120),
        }
    }

    pub fn poll_telemetry(&mut self) {
        let mut batch = Vec::new();
        if let Some(ref rx) = self.telem_rx {
            while let Ok(telem) = rx.try_recv() {
                batch.push(telem);
            }
        }
        for telem in batch {
            self.update_telemetry(telem);
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

        const MAX_HISTORY: usize = 100;
        self.gyro_history_x.push_back(telem.gyro_x as f32);
        if self.gyro_history_x.len() > MAX_HISTORY {
            self.gyro_history_x.pop_front();
        }
        self.gyro_history_y.push_back(telem.gyro_y as f32);
        if self.gyro_history_y.len() > MAX_HISTORY {
            self.gyro_history_y.pop_front();
        }
        self.accel_history_x.push_back(telem.accel_x as f32);
        if self.accel_history_x.len() > MAX_HISTORY {
            self.accel_history_x.pop_front();
        }
        self.accel_history_y.push_back(telem.accel_y as f32);
        if self.accel_history_y.len() > MAX_HISTORY {
            self.accel_history_y.pop_front();
        }
        self.accel_history_z.push_back(telem.accel_z as f32);
        if self.accel_history_z.len() > MAX_HISTORY {
            self.accel_history_z.pop_front();
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

    pub fn update_background(&mut self, ctx: &egui::Context) {
        if self.should_exit {
            let _ = self.driver_cmd_tx.send(DriverCommand::Stop);
            std::process::exit(0);
        }

        self.poll_telemetry();

        // Process System Tray events & live connection status
        if let Some(ref mut tray) = self.tray {
            tray.update_status(self.latest_telemetry.connected, &self.latest_telemetry.port_name);
            match tray.process_events(&self.driver_cmd_tx) {
                Some(TrayAction::OpenPanel) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                Some(TrayAction::Quit) => {
                    self.should_exit = true;
                    let _ = self.driver_cmd_tx.send(DriverCommand::Stop);
                    std::process::exit(0);
                }
                None => {}
            }
        }

        // Intercept close button to hide to tray (unless user clicked Sair)
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.should_exit {
                let _ = self.driver_cmd_tx.send(DriverCommand::Stop);
                std::process::exit(0);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }

        if self.active_tab == Tab::Sensors {
            ctx.request_repaint();
        } else if self.latest_telemetry.connected || !self.latest_telemetry.active_buttons.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(30));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.ensure_texture(&ctx);

        // Top Header Panel
        Panel::top("header").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("LG Magic Motion Painel");

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
                        ui.selectable_value(&mut self.active_tab, Tab::Buttons, "Botões");
                        ui.selectable_value(&mut self.active_tab, Tab::Motion, "Movimento");
                        ui.selectable_value(&mut self.active_tab, Tab::Settings, "Configurações");
                        ui.selectable_value(&mut self.active_tab, Tab::Sensors, "Dados Brutos");
                    });
                    ui.separator();

                    match self.active_tab {
                        Tab::Buttons => self.render_buttons_tab(ui),
                        Tab::Motion => self.render_motion_tab(ui),
                        Tab::Settings => self.render_settings_tab(ui),
                        Tab::Sensors => self.render_sensors_tab(ui),
                    }
                });
            });
        });
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
        ui.heading("Ajustes de Movimento e Sensibilidade");
        ui.label(
            egui::RichText::new("Curvas e multiplicadores de resposta para navegação no desktop e mira em jogos.")
                .small()
                .color(Color32::GRAY),
        );
        ui.add_space(8.0);

        let mut changed = false;

        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            ui.group(|ui| {
                ui.label(egui::RichText::new("Sensibilidade dos Eixos:").strong());
                changed |= ui.add(egui::Slider::new(&mut self.config.motion.sensitivity, 0.05..=1.5).text("Sensibilidade Geral")).changed();
                changed |= ui.add(egui::Slider::new(&mut self.config.motion.sensitivity_x, 0.2..=2.5).text("Multiplicador X (Yaw / Horizontal)")).changed();
                changed |= ui.add(egui::Slider::new(&mut self.config.motion.sensitivity_y, 0.2..=2.5).text("Multiplicador Y (Pitch / Vertical)")).changed();
            });

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Precisão & Filtros:").strong());
                changed |= ui.add(egui::Slider::new(&mut self.config.motion.deadzone, 0.0..=6.0).text("Zona Morta (Deadzone)").suffix(" pts")).changed();
                ui.label(egui::RichText::new("Elimina micro-tremores e ruído do sensor com a mão parada.").small().color(Color32::GRAY));

                ui.add_space(4.0);
                changed |= ui.add(egui::Slider::new(&mut self.config.motion.acceleration, 1.0..=1.8).text("Curva de Aceleração")).changed();
                ui.label(egui::RichText::new("1.0 = Movimento linear (1:1). Acima de 1.1 dá mais agilidade a movimentos rápidos de pulso.").small().color(Color32::GRAY));

                ui.add_space(4.0);
                changed |= ui.add(egui::Slider::new(&mut self.config.motion.smoothing, 0.0..=0.75).text("Suavização (Low-Pass Filter)")).changed();
                ui.label(egui::RichText::new("Filtro exponencial que suaviza os passos do mouse sem adicionar atraso perceptível.").small().color(Color32::GRAY));
            });

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Inversão de Eixos:").strong());
                changed |= ui.checkbox(&mut self.config.motion.invert_x, "Inverter Eixo X (Horizontal)").changed();
                changed |= ui.checkbox(&mut self.config.motion.invert_y, "Inverter Eixo Y (Vertical)").changed();
            });

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label(egui::RichText::new("Orientação & Compensação de Rotação:").strong());
                changed |= ui.checkbox(
                    &mut self.config.motion.roll_compensation,
                    "Compensar Rotação do Controle (Roll / Gravidade)",
                ).changed();
                ui.label(
                    egui::RichText::new("Garante que apontar para cima sempre mova o mouse para cima, mesmo segurando o controle inclinado ou deitado de lado.")
                        .small()
                        .color(Color32::GRAY),
                );
            });
        });

        if changed {
            let _ = self.driver_cmd_tx.send(DriverCommand::UpdateConfig(self.config.clone()));
        }
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configurações do Sistema e Dispositivo");
        ui.label(
            egui::RichText::new("Gerencie telas, inicialização automática, conexões e calibração de sensores.")
                .small()
                .color(Color32::GRAY),
        );
        ui.add_space(8.0);

        let mut config_changed = false;

        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            // 1. Monitores & Exibição
            ui.group(|ui| {
                ui.label(egui::RichText::new("Monitores & Área de Controle:").strong());
                ui.label(
                    egui::RichText::new("Permite confinar os movimentos do Magic Remote a um monitor específico em computadores com múltiplas telas.")
                        .small()
                        .color(Color32::GRAY),
                );

                let monitors = crate::infra::input::list_monitors();
                let current_monitor_label = match self.config.target_monitor {
                    Some(id) => {
                        if let Some(m) = monitors.iter().find(|m| m.id == id) {
                            format!("{} ({}x{})", m.name, m.rect[2] - m.rect[0], m.rect[3] - m.rect[1])
                        } else {
                            format!("Monitor {id}")
                        }
                    }
                    None => "Todos os Monitores (Livre)".to_string(),
                };

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Restringir a:");
                    egui::ComboBox::from_id_salt("settings_monitor_selector")
                        .selected_text(&current_monitor_label)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(self.config.target_monitor.is_none(), "Todos os Monitores (Livre)").clicked() && self.config.target_monitor.is_some() {
                                self.config.target_monitor = None;
                                config_changed = true;
                            }
                            for m in &monitors {
                                let is_selected = self.config.target_monitor == Some(m.id);
                                let label = format!("{} ({}x{})", m.name, m.rect[2] - m.rect[0], m.rect[3] - m.rect[1]);
                                if ui.selectable_label(is_selected, label).clicked() && !is_selected {
                                    self.config.target_monitor = Some(m.id);
                                    config_changed = true;
                                }
                            }
                        });
                });
            });

            ui.add_space(8.0);

            // 2. Sistema & Inicialização
            ui.group(|ui| {
                ui.label(egui::RichText::new("Sistema & Inicialização:").strong());
                let mut autostart = is_autostart_enabled();
                if ui.checkbox(&mut autostart, "Iniciar automaticamente junto com o Windows").changed() {
                    let _ = set_autostart(autostart);
                    self.config.auto_start = autostart;
                    self.save_config();
                }
                ui.label(
                    egui::RichText::new("Ao fechar esta janela no 'X', o driver continuará em segundo plano na bandeja do sistema.")
                        .small()
                        .color(Color32::GRAY),
                );
            });

            ui.add_space(8.0);

            // 3. Conexão & Porta Serial
            ui.group(|ui| {
                ui.label(egui::RichText::new("Conexão e Porta Serial (COM):").strong());
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
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Modo da Porta:");
                    egui::ComboBox::from_id_salt("settings_port_selector")
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
                });

                if port_changed {
                    self.save_config();
                }
            });

            ui.add_space(8.0);

            // 4. Calibração de Ponto Neutro (Drift do Giroscópio)
            ui.group(|ui| {
                ui.label(egui::RichText::new("Calibração de Ponto Neutro (Zero do Giroscópio):").strong());
                ui.label(
                    egui::RichText::new(
                        "O giroscópio mede a velocidade angular da sua mão. Se com o controle parado sobre a mesa \
                        o cursor do mouse estiver se deslocando vagarosamente sozinho (drift), coloque o controle \
                        imóvel sobre uma superfície plana e clique em 'Calibrar Ponto Neutro Atual'.",
                    )
                    .small()
                    .color(Color32::GRAY),
                );

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Centro calibrado atual: X={:.1}, Y={:.1} (Padrão de fábrica: 537.0)",
                        self.latest_telemetry.center_x, self.latest_telemetry.center_y
                    ));
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Calibrar Ponto Neutro Atual").clicked() {
                        let _ = self.driver_cmd_tx.send(DriverCommand::Recenter);
                        self.calibration_feedback = Some((
                            format!(
                                "Calibrado com a posição atual (X={}, Y={})!",
                                self.latest_telemetry.gyro_x, self.latest_telemetry.gyro_y
                            ),
                            std::time::Instant::now(),
                        ));
                    }

                    if ui.button("Restaurar Centro Padrão (537)").clicked() {
                        let _ = self.driver_cmd_tx.send(DriverCommand::ResetCenter);
                        self.calibration_feedback = Some((
                            "Centro restaurado para o padrão de hardware (537.0).".to_string(),
                            std::time::Instant::now(),
                        ));
                    }
                });

                if let Some((ref msg, time)) = self.calibration_feedback {
                    if time.elapsed() < std::time::Duration::from_secs(4) {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new(msg).color(Color32::from_rgb(100, 220, 140)));
                    }
                }
            });

            ui.add_space(8.0);

            // 5. Restauração de Fábrica
            ui.group(|ui| {
                ui.label(egui::RichText::new("Restaurar Padrões:").strong());
                ui.label(
                    egui::RichText::new("Restaura todos os mapeamentos de botões, sensibilidade e opções para as configurações de fábrica.")
                        .small()
                        .color(Color32::GRAY),
                );
                ui.add_space(4.0);
                if ui.button("Restaurar Todas as Configurações de Fábrica").clicked() {
                    self.config = Config::default();
                    self.save_config();
                    let _ = self.driver_cmd_tx.send(DriverCommand::ResetCenter);
                }
            });
        });

        if config_changed {
            self.save_config();
        }
    }

    fn render_sensors_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dados Brutos do Controle em Tempo Real");
        ui.label(
            egui::RichText::new("Leituras diretas dos conversores analógico-digitais (ADC) e sensores a ~111 Hz.")
                .small()
                .color(Color32::GRAY),
        );
        ui.add_space(6.0);

        let t = &self.latest_telemetry;

        egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            // 1. Conexão & Telemetria
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Status da Conexão:").strong());
                    if t.connected {
                        ui.label(egui::RichText::new("CONECTADO").color(Color32::from_rgb(40, 220, 100)).strong());
                    } else {
                        ui.label(egui::RichText::new("DESCONECTADO").color(Color32::from_rgb(230, 60, 60)).strong());
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{:.1} Hz (~{:.1} ms)", t.rate_hz, if t.rate_hz > 0.0 { 1000.0 / t.rate_hz } else { 0.0 })).strong());
                    });
                });

                ui.horizontal(|ui| {
                    let port_txt = if t.port_name.is_empty() { "Nenhuma" } else { &t.port_name };
                    ui.label(format!("Porta COM: {port_txt}"));
                    ui.separator();
                    let cursor_status = if t.cursor_enabled { "Ativa" } else { "Pausada" };
                    ui.label(format!("Mira: {cursor_status}"));
                    ui.separator();
                    ui.label(format!("Inclinação (Roll): {:.1}°", t.roll_deg));
                });
            });

            ui.add_space(6.0);

            // 2. Giroscópio com Gráficos de Slide e Osciloscópio
            ui.group(|ui| {
                ui.label(egui::RichText::new("Giroscópio (Velocidade Angular):").strong());
                ui.label(
                    egui::RichText::new("O giroscópio mede rotação angular. Valores próximos ao centro (537) indicam repouso.")
                        .small()
                        .color(Color32::GRAY),
                );
                ui.add_space(4.0);

                let delta_x_str = format!("Delta: {:+3}", t.gyro_dx);
                draw_bipolar_slide(
                    ui,
                    "Eixo X (Yaw / Horizontal):",
                    t.gyro_x as f32,
                    t.center_x,
                    350.0,
                    750.0,
                    &delta_x_str,
                    Color32::from_rgb(0, 210, 255),
                );

                let delta_y_str = format!("Eixo Y (Pitch / Vertical):");
                let delta_y_val = format!("Delta: {:+3}", t.gyro_dy);
                draw_bipolar_slide(
                    ui,
                    &delta_y_str,
                    t.gyro_y as f32,
                    t.center_y,
                    350.0,
                    750.0,
                    &delta_y_val,
                    Color32::from_rgb(255, 195, 30),
                );

                ui.add_space(4.0);
                draw_oscilloscope(
                    ui,
                    &self.gyro_history_x,
                    &self.gyro_history_y,
                    t.center_x,
                    t.center_y,
                );
            });

            ui.add_space(6.0);

            // 3. Acelerômetro com Gráficos de Slide
            ui.group(|ui| {
                ui.label(egui::RichText::new("Acelerômetro (Gravidade & Inclinação 3D):").strong());
                ui.label(
                    egui::RichText::new("Mede os vetores de força G nos 3 eixos espaciais (ADC 12-bit, repouso ~2048).")
                        .small()
                        .color(Color32::GRAY),
                );
                ui.add_space(4.0);

                let [g_x, g_y, g_z] = crate::domain::gravity::AccelCalibration::LG_AN_MR200
                    .to_g([t.accel_x, t.accel_y, t.accel_z]);
                let total_g = (g_x * g_x + g_y * g_y + g_z * g_z).sqrt();

                ui.label(format!("Força G Total: {:.2}g (Repouso: 1.00g)", total_g));
                ui.add_space(2.0);

                draw_bipolar_slide(
                    ui,
                    "Aceleração X (Longitudinal / Frente):",
                    t.accel_x as f32,
                    2055.0,
                    1000.0,
                    3100.0,
                    &format!("({:+.2}g)", g_x),
                    Color32::from_rgb(244, 63, 94),
                );

                draw_bipolar_slide(
                    ui,
                    "Aceleração Y (Lateral / Esquerda-Direita):",
                    t.accel_y as f32,
                    2012.0,
                    1000.0,
                    3100.0,
                    &format!("({:+.2}g)", g_y),
                    Color32::from_rgb(34, 197, 94),
                );

                draw_bipolar_slide(
                    ui,
                    "Aceleração Z (Normal / Botões):",
                    t.accel_z as f32,
                    2180.0,
                    1000.0,
                    3100.0,
                    &format!("({:+.2}g)", g_z),
                    Color32::from_rgb(14, 165, 233),
                );
            });

            ui.add_space(6.0);

            // 4. Botões Físicos Pressionados
            ui.group(|ui| {
                ui.label(egui::RichText::new("Estado Físico dos Botões (Tempo Real):").strong());
                if t.active_buttons.is_empty() {
                    ui.label(egui::RichText::new("Nenhum botão pressionado no controle").color(Color32::GRAY));
                } else {
                    ui.horizontal_wrapped(|ui| {
                        for b in t.active_buttons.iter() {
                            ui.label(
                                egui::RichText::new(format!("● {:?}", b))
                                    .color(Color32::from_rgb(0, 255, 180))
                                    .strong(),
                            );
                        }
                    });
                }
            });
            ui.add_space(8.0);
        });
    }
}

fn draw_bipolar_slide(
    ui: &mut egui::Ui,
    label: &str,
    val: f32,
    center: f32,
    min_range: f32,
    max_range: f32,
    extra_info: &str,
    bar_color: Color32,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).strong());
        if !extra_info.is_empty() {
            ui.label(egui::RichText::new(extra_info).color(Color32::from_rgb(160, 170, 190)).small());
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(format!("{:.0}", val)).strong().monospace());
        });
    });

    let height = 15.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Track background
    painter.rect_filled(rect, 3.0, Color32::from_rgb(24, 27, 34));
    painter.rect_stroke(rect, 3.0, Stroke::new(1.0, Color32::from_rgb(42, 48, 60)), StrokeKind::Inside);

    // Center marker
    let norm_center = ((center - min_range) / (max_range - min_range)).clamp(0.0, 1.0);
    let center_x = rect.min.x + rect.width() * norm_center;
    painter.line_segment(
        [Pos2::new(center_x, rect.min.y), Pos2::new(center_x, rect.max.y)],
        Stroke::new(1.5, Color32::from_rgb(90, 105, 125)),
    );

    // Value bar
    let clamped_val = val.clamp(min_range, max_range);
    let norm_val = (clamped_val - min_range) / (max_range - min_range);
    let val_x = rect.min.x + rect.width() * norm_val;

    if (val_x - center_x).abs() > 1.0 {
        let (x_start, x_end) = if val_x < center_x { (val_x, center_x) } else { (center_x, val_x) };
        let bar_rect = Rect::from_min_max(Pos2::new(x_start, rect.min.y + 2.0), Pos2::new(x_end, rect.max.y - 2.0));
        painter.rect_filled(bar_rect, 2.0, bar_color);
    }

    // Pin indicator
    painter.line_segment(
        [Pos2::new(val_x, rect.min.y - 1.0), Pos2::new(val_x, rect.max.y + 1.0)],
        Stroke::new(2.5, Color32::WHITE),
    );
    ui.add_space(3.0);
}

fn draw_oscilloscope(
    ui: &mut egui::Ui,
    history_x: &VecDeque<f32>,
    history_y: &VecDeque<f32>,
    center_x: f32,
    center_y: f32,
) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Gráfico Osciloscópio (Tempo Real):").strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new("— Giro Y (Pitch)").color(Color32::from_rgb(255, 195, 30)).small());
            ui.add_space(8.0);
            ui.label(egui::RichText::new("— Giro X (Yaw)").color(Color32::from_rgb(0, 210, 255)).small());
        });
    });

    let height = 80.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 4.0, Color32::from_rgb(14, 17, 23));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0, Color32::from_rgb(38, 46, 62)), StrokeKind::Inside);

    // Midline
    let mid_y = rect.min.y + rect.height() * 0.5;
    painter.line_segment(
        [Pos2::new(rect.min.x, mid_y), Pos2::new(rect.max.x, mid_y)],
        Stroke::new(1.0, Color32::from_rgb(30, 40, 56)),
    );

    let n = history_x.len();
    if n >= 2 {
        let span = 120.0f32; // +/- 120 counts from center
        let step_x = rect.width() / (n - 1) as f32;

        let points_x: Vec<Pos2> = history_x
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let px = rect.min.x + i as f32 * step_x;
                let py = mid_y - ((v - center_x) / span) * (rect.height() * 0.44);
                Pos2::new(px, py.clamp(rect.min.y + 2.0, rect.max.y - 2.0))
            })
            .collect();

        for w in points_x.windows(2) {
            painter.line_segment([w[0], w[1]], Stroke::new(1.8, Color32::from_rgb(0, 210, 255)));
        }

        let points_y: Vec<Pos2> = history_y
            .iter()
            .enumerate()
            .map(|(i, &v)| {
                let px = rect.min.x + i as f32 * step_x;
                let py = mid_y - ((v - center_y) / span) * (rect.height() * 0.44);
                Pos2::new(px, py.clamp(rect.min.y + 2.0, rect.max.y - 2.0))
            })
            .collect();

        for w in points_y.windows(2) {
            painter.line_segment([w[0], w[1]], Stroke::new(1.8, Color32::from_rgb(255, 195, 30)));
        }
    }
}
