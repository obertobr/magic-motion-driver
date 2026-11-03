//! Presentation layer internationalization strings and catalogs.

use crate::domain::i18n::Language;

#[derive(Clone, Copy, Debug)]
pub struct Messages {
    // Header & Tabs
    pub app_title: &'static str,
    pub status_connected: &'static str,
    pub status_disconnected: &'static str,
    pub cursor_active: &'static str,
    pub cursor_paused: &'static str,
    pub tab_buttons: &'static str,
    pub tab_motion: &'static str,
    pub tab_settings: &'static str,
    pub tab_raw_data: &'static str,

    // Footer
    pub save_settings: &'static str,
    pub saved_success: &'static str,

    // Buttons Tab
    pub configure_button_action: &'static str,
    pub restore_default_buttons: &'static str,
    pub shortcut_record_prompt: &'static str,
    pub shortcut_record_idle: &'static str,
    pub cat_mouse: &'static str,
    pub cat_keyboard: &'static str,
    pub cat_media: &'static str,
    pub cat_system: &'static str,
    pub mouse_left: &'static str,
    pub mouse_right: &'static str,
    pub mouse_middle: &'static str,
    pub mouse_wheel_up: &'static str,
    pub mouse_wheel_down: &'static str,
    pub key_win: &'static str,
    pub key_enter: &'static str,
    pub key_esc: &'static str,
    pub key_space: &'static str,
    pub key_backspace: &'static str,
    pub key_tab: &'static str,
    pub key_up: &'static str,
    pub key_down: &'static str,
    pub key_left: &'static str,
    pub key_right: &'static str,
    pub act_toggle_cursor: &'static str,
    pub act_recenter: &'static str,
    pub act_disable: &'static str,

    // Motion Tab
    pub motion_heading: &'static str,
    pub sec_axis_sensitivity: &'static str,
    pub overall_sensitivity: &'static str,
    pub multiplier_x: &'static str,
    pub multiplier_y: &'static str,
    pub sec_precision_filters: &'static str,
    pub deadzone: &'static str,
    pub deadzone_desc: &'static str,
    pub acceleration: &'static str,
    pub acceleration_desc: &'static str,
    pub smoothing: &'static str,
    pub smoothing_desc: &'static str,
    pub sec_inversion: &'static str,
    pub invert_x: &'static str,
    pub invert_y: &'static str,
    pub sec_orientation_roll: &'static str,
    pub roll_compensation: &'static str,
    pub roll_compensation_desc: &'static str,

    // Settings Tab
    pub settings_heading: &'static str,
    pub sec_language: &'static str,
    pub language_label: &'static str,
    pub sec_monitors: &'static str,
    pub monitors_desc: &'static str,
    pub restrict_to: &'static str,
    pub all_monitors: &'static str,
    pub sec_system_startup: &'static str,
    pub autostart_checkbox: &'static str,
    pub autostart_desc: &'static str,
    pub sec_connection_com: &'static str,
    pub connection_desc: &'static str,
    pub port_mode: &'static str,
    pub port_auto: &'static str,
    pub port_force_prefix: &'static str,
    pub sec_gyro_calibration: &'static str,
    pub gyro_calibration_desc: &'static str,
    pub calibrate_now_btn: &'static str,
    pub restore_center_btn: &'static str,
    pub center_restored_msg: &'static str,
    pub sec_restore_defaults: &'static str,
    pub restore_defaults_desc: &'static str,
    pub restore_defaults_btn: &'static str,

    // Raw Data Tab
    pub raw_data_heading: &'static str,
    pub conn_status: &'static str,
    pub com_port_prefix: &'static str,
    pub port_none: &'static str,
    pub tilt_roll: &'static str,
    pub gyroscope: &'static str,
    pub gyro_axis_x: &'static str,
    pub gyro_axis_y: &'static str,
    pub accelerometer: &'static str,
    pub accel_axis_x: &'static str,
    pub accel_axis_y: &'static str,
    pub accel_axis_z: &'static str,
    pub sensor_graph: &'static str,
    pub record_start_btn: &'static str,
    pub record_start_tooltip: &'static str,
    pub record_stop_btn: &'static str,
    pub record_folder_tooltip: &'static str,
    pub record_none_msg: &'static str,
    pub channels: &'static str,
    pub channel_gyro_x: &'static str,
    pub channel_gyro_y: &'static str,
    pub channel_accel_x: &'static str,
    pub channel_accel_y: &'static str,
    pub channel_accel_z: &'static str,
    pub view_3d: &'static str,
    pub center_3d_btn: &'static str,
    pub center_3d_tooltip: &'static str,
    pub pressed_button: &'static str,
    pub no_button_pressed: &'static str,

    // Tray Menu
    pub tray_open_panel: &'static str,
    pub tray_toggle_cursor: &'static str,
    pub tray_quit: &'static str,
}

impl Messages {
    pub fn tooltip_connected(&self, port: &str, rate_hz: f32) -> String {
        format!("{}: {} ({}) - {:.0} Hz", self.app_title, self.status_connected, port, rate_hz)
    }

    pub fn tooltip_disconnected(&self, lang: Language) -> &'static str {
        match lang {
            Language::Pt => "Desconectado (Aguardando receptor CP2102...)",
            Language::En => "Disconnected (Waiting for CP2102 receiver...)",
        }
    }

    pub fn shortcut_custom_label(&self, s_label: &str, lang: Language) -> String {
        match lang {
            Language::Pt => format!("⌨ Atalho Personalizado: {s_label}  (Clique para alterar)"),
            Language::En => format!("⌨ Custom Shortcut: {s_label}  (Click to change)"),
        }
    }

    pub fn center_calibrated_label(&self, cx: f32, cy: f32, lang: Language) -> String {
        match lang {
            Language::Pt => format!("Centro calibrado atual: X={cx:.1}, Y={cy:.1} (Padrão de fábrica: 537.0)"),
            Language::En => format!("Current calibrated center: X={cx:.1}, Y={cy:.1} (Factory default: 537.0)"),
        }
    }

    pub fn calibrated_success_msg(&self, x: u16, y: u16, lang: Language) -> String {
        match lang {
            Language::Pt => format!("Calibrado com a posição atual (X={x}, Y={y})!"),
            Language::En => format!("Calibrated with current position (X={x}, Y={y})!"),
        }
    }

    pub fn recording_active_msg(&self, count: usize, elapsed: f32, lang: Language) -> String {
        match lang {
            Language::Pt => format!("● Gravando ({count} amostras, {elapsed:.1}s)"),
            Language::En => format!("● Recording ({count} samples, {elapsed:.1}s)"),
        }
    }

    pub fn recording_saved_msg(&self, filename: &str, count: usize, lang: Language) -> String {
        match lang {
            Language::Pt => format!("Salvo com sucesso: {filename} ({count} amostras)"),
            Language::En => format!("Successfully saved: {filename} ({count} samples)"),
        }
    }
}

pub static PT_MESSAGES: Messages = Messages {
    app_title: "LG Magic Motion Painel",
    status_connected: "CONECTADO",
    status_disconnected: "DESCONECTADO",
    cursor_active: "Mira: ATIVA",
    cursor_paused: "Mira: PAUSADA",
    tab_buttons: "Botões",
    tab_motion: "Movimento",
    tab_settings: "Configurações",
    tab_raw_data: "Dados Brutos",

    save_settings: "Salvar Configurações",
    saved_success: "Configurações salvas com sucesso!",

    configure_button_action: "Configurar Ação do Botão",
    restore_default_buttons: "Restaurar Botões Padrão",
    shortcut_record_prompt: "⌨ Pressione a tecla ou atalho... (Clique para cancelar)",
    shortcut_record_idle: "⌨ Gravar Tecla ou Atalho Personalizado...",
    cat_mouse: "Mouse",
    cat_keyboard: "Teclado",
    cat_media: "Mídia",
    cat_system: "Sistema",
    mouse_left: "Clique Esquerdo",
    mouse_right: "Clique Direito",
    mouse_middle: "Clique do Meio",
    mouse_wheel_up: "Rolar para Cima",
    mouse_wheel_down: "Rolar para Baixo",
    key_win: "Tecla Windows (Win)",
    key_enter: "Enter",
    key_esc: "Esc",
    key_space: "Espaço",
    key_backspace: "Backspace",
    key_tab: "Tab",
    key_up: "Seta Cima",
    key_down: "Seta Baixo",
    key_left: "Seta Esquerda",
    key_right: "Seta Direita",
    act_toggle_cursor: "Ativar / Pausar Mira",
    act_recenter: "Recalibrar Centro",
    act_disable: "Desativar Botão",

    motion_heading: "Ajustes de Movimento e Sensibilidade",
    sec_axis_sensitivity: "Sensibilidade dos Eixos:",
    overall_sensitivity: "Sensibilidade Geral",
    multiplier_x: "Multiplicador X (Yaw / Horizontal)",
    multiplier_y: "Multiplicador Y (Pitch / Vertical)",
    sec_precision_filters: "Precisão & Filtros:",
    deadzone: "Zona Morta (Deadzone)",
    deadzone_desc: "Elimina micro-tremores e ruído do sensor com a mão parada.",
    acceleration: "Curva de Aceleração",
    acceleration_desc: "1.0 = Movimento linear (1:1). Acima de 1.1 dá mais agilidade a movimentos rápidos de pulso.",
    smoothing: "Suavização (Low-Pass Filter)",
    smoothing_desc: "Filtro exponencial que suaviza os passos do mouse sem adicionar atraso perceptível.",
    sec_inversion: "Inversão de Eixos:",
    invert_x: "Inverter Eixo X (Horizontal)",
    invert_y: "Inverter Eixo Y (Vertical)",
    sec_orientation_roll: "Orientação & Compensação de Rotação:",
    roll_compensation: "Compensar Rotação do Controle (Roll / Gravidade)",
    roll_compensation_desc: "Garante que apontar para cima sempre mova o mouse para cima, mesmo segurando o controle inclinado ou deitado de lado.",

    settings_heading: "Configurações do Sistema e Dispositivo",
    sec_language: "Idioma da Interface:",
    language_label: "Idioma:",
    sec_monitors: "Monitores & Área de Controle:",
    monitors_desc: "Permite confinar os movimentos do Magic Remote a um monitor específico em computadores com múltiplas telas.",
    restrict_to: "Restringir a:",
    all_monitors: "Todos os Monitores (Livre)",
    sec_system_startup: "Sistema & Inicialização:",
    autostart_checkbox: "Iniciar automaticamente junto com o Windows",
    autostart_desc: "Ao fechar esta janela no 'X', o driver continuará em segundo plano na bandeja do sistema.",
    sec_connection_com: "Conexão e Porta Serial (COM):",
    connection_desc: "Por padrão, o driver detecta automaticamente a porta do receptor CP2102 da LG.",
    port_mode: "Modo da Porta:",
    port_auto: "Automático (Auto-detectar receptor CP2102)",
    port_force_prefix: "Forçar",
    sec_gyro_calibration: "Calibração de Ponto Neutro (Zero do Giroscópio):",
    gyro_calibration_desc: "O giroscópio mede a velocidade angular da sua mão. Se com o controle parado sobre a mesa o cursor do mouse estiver se deslocando vagarosamente sozinho (drift), coloque o controle imóvel sobre uma superfície plana e clique em 'Calibrar Ponto Neutro Atual'.",
    calibrate_now_btn: "Calibrar Ponto Neutro Atual",
    restore_center_btn: "Restaurar Centro Padrão (537)",
    center_restored_msg: "Centro restaurado para o padrão de hardware (537.0).",
    sec_restore_defaults: "Restaurar Padrões:",
    restore_defaults_desc: "Restaura todos os mapeamentos de botões, sensibilidade e opções para as configurações de fábrica.",
    restore_defaults_btn: "Restaurar Todas as Configurações de Fábrica",

    raw_data_heading: "Dados Brutos do Controle em Tempo Real",
    conn_status: "Status da Conexão",
    com_port_prefix: "Porta COM:",
    port_none: "Nenhuma",
    tilt_roll: "Inclinação (Roll):",
    gyroscope: "Giroscópio",
    gyro_axis_x: "Eixo X (Yaw):",
    gyro_axis_y: "Eixo Y (Pitch):",
    accelerometer: "Acelerômetro",
    accel_axis_x: "Acel X (Frente):",
    accel_axis_y: "Acel Y (Lateral):",
    accel_axis_z: "Acel Z (Botões):",
    sensor_graph: "Gráfico de Sensores",
    record_start_btn: "⏺ Gravar Dados",
    record_start_tooltip: "Inicia a gravação de todas as leituras a ~111 Hz em arquivo CSV",
    record_stop_btn: "⏹ Parar e Salvar",
    record_folder_tooltip: "Abrir pasta onde as gravações CSV são salvas",
    record_none_msg: "Nenhum dado gravado.",
    channels: "Canais:",
    channel_gyro_x: "Giro X (Yaw)",
    channel_gyro_y: "Giro Y (Pitch)",
    channel_accel_x: "Acel X (Frente)",
    channel_accel_y: "Acel Y (Lateral)",
    channel_accel_z: "Acel Z (Botões)",
    view_3d: "Visualização 3D",
    center_3d_btn: "⟲ Centralizar",
    center_3d_tooltip: "Define a posição física atual do controle como centro neutro de referência",
    pressed_button: "Botão Pressionado:",
    no_button_pressed: "Nenhum botão pressionado",

    tray_open_panel: "Abrir Painel",
    tray_toggle_cursor: "Ativar / Pausar Mira",
    tray_quit: "Sair",
};

pub static EN_MESSAGES: Messages = Messages {
    app_title: "LG Magic Motion Panel",
    status_connected: "CONNECTED",
    status_disconnected: "DISCONNECTED",
    cursor_active: "Aim: ACTIVE",
    cursor_paused: "Aim: PAUSED",
    tab_buttons: "Buttons",
    tab_motion: "Motion",
    tab_settings: "Settings",
    tab_raw_data: "Raw Data",

    save_settings: "Save Settings",
    saved_success: "Settings saved successfully!",

    configure_button_action: "Configure Button Action",
    restore_default_buttons: "Restore Default Buttons",
    shortcut_record_prompt: "⌨ Press key or shortcut... (Click to cancel)",
    shortcut_record_idle: "⌨ Record Custom Key or Shortcut...",
    cat_mouse: "Mouse",
    cat_keyboard: "Keyboard",
    cat_media: "Media",
    cat_system: "System",
    mouse_left: "Left Click",
    mouse_right: "Right Click",
    mouse_middle: "Middle Click",
    mouse_wheel_up: "Scroll Up",
    mouse_wheel_down: "Scroll Down",
    key_win: "Windows Key (Win)",
    key_enter: "Enter",
    key_esc: "Esc",
    key_space: "Space",
    key_backspace: "Backspace",
    key_tab: "Tab",
    key_up: "Up Arrow",
    key_down: "Down Arrow",
    key_left: "Left Arrow",
    key_right: "Right Arrow",
    act_toggle_cursor: "Toggle Cursor / Aim",
    act_recenter: "Recalibrate Center",
    act_disable: "Disable Button",

    motion_heading: "Motion & Sensitivity Settings",
    sec_axis_sensitivity: "Axis Sensitivity:",
    overall_sensitivity: "Overall Sensitivity",
    multiplier_x: "X Multiplier (Yaw / Horizontal)",
    multiplier_y: "Y Multiplier (Pitch / Vertical)",
    sec_precision_filters: "Precision & Filters:",
    deadzone: "Deadzone",
    deadzone_desc: "Eliminates micro-tremors and sensor noise when stationary.",
    acceleration: "Acceleration Curve",
    acceleration_desc: "1.0 = Linear motion (1:1). Above 1.1 gives more agility for quick wrist flicks.",
    smoothing: "Smoothing (Low-Pass Filter)",
    smoothing_desc: "Exponential filter that smooths mouse steps without noticeable lag.",
    sec_inversion: "Axis Inversion:",
    invert_x: "Invert X Axis (Horizontal)",
    invert_y: "Invert Y Axis (Vertical)",
    sec_orientation_roll: "Orientation & Roll Compensation:",
    roll_compensation: "Compensate Remote Rotation (Roll / Gravity)",
    roll_compensation_desc: "Ensures pointing upward always moves cursor upward, even when tilting or holding the remote sideways.",

    settings_heading: "System and Device Settings",
    sec_language: "Interface Language:",
    language_label: "Language:",
    sec_monitors: "Monitors & Display Area:",
    monitors_desc: "Allows constraining Magic Remote pointer movements to a specific monitor in multi-display setups.",
    restrict_to: "Constrain to:",
    all_monitors: "All Monitors (Unconstrained)",
    sec_system_startup: "System & Startup:",
    autostart_checkbox: "Start automatically with Windows",
    autostart_desc: "When closing this window with 'X', the driver will continue running in the background in the system tray.",
    sec_connection_com: "Connection & Serial Port (COM):",
    connection_desc: "By default, the driver automatically detects the LG CP2102 receiver COM port.",
    port_mode: "Port Mode:",
    port_auto: "Automatic (Auto-detect CP2102 receiver)",
    port_force_prefix: "Force",
    sec_gyro_calibration: "Neutral Point Calibration (Gyroscope Zero):",
    gyro_calibration_desc: "The gyroscope measures your hand's angular velocity. If the cursor slowly drifts on its own while the remote is resting on a table, place the remote stationary on a flat surface and click 'Calibrate Current Neutral Point'.",
    calibrate_now_btn: "Calibrate Current Neutral Point",
    restore_center_btn: "Restore Default Center (537)",
    center_restored_msg: "Center restored to hardware default (537.0).",
    sec_restore_defaults: "Restore Defaults:",
    restore_defaults_desc: "Restores all button mappings, sensitivity curves, and preferences to factory default settings.",
    restore_defaults_btn: "Restore All Factory Settings",

    raw_data_heading: "Real-Time Remote Raw Data",
    conn_status: "Connection Status",
    com_port_prefix: "COM Port:",
    port_none: "None",
    tilt_roll: "Tilt (Roll):",
    gyroscope: "Gyroscope",
    gyro_axis_x: "X Axis (Yaw):",
    gyro_axis_y: "Y Axis (Pitch):",
    accelerometer: "Accelerometer",
    accel_axis_x: "Accel X (Front):",
    accel_axis_y: "Accel Y (Side):",
    accel_axis_z: "Accel Z (Buttons):",
    sensor_graph: "Sensor Graph",
    record_start_btn: "⏺ Record Data",
    record_start_tooltip: "Starts recording all readings at ~111 Hz to a CSV file",
    record_stop_btn: "⏹ Stop & Save",
    record_folder_tooltip: "Open folder where CSV recordings are saved",
    record_none_msg: "No data recorded.",
    channels: "Channels:",
    channel_gyro_x: "Gyro X (Yaw)",
    channel_gyro_y: "Gyro Y (Pitch)",
    channel_accel_x: "Accel X (Front)",
    channel_accel_y: "Accel Y (Side)",
    channel_accel_z: "Accel Z (Buttons)",
    view_3d: "3D View",
    center_3d_btn: "⟲ Center",
    center_3d_tooltip: "Sets remote's current physical position as the neutral reference center",
    pressed_button: "Pressed Button:",
    no_button_pressed: "No button pressed",

    tray_open_panel: "Open Panel",
    tray_toggle_cursor: "Toggle Cursor / Aim",
    tray_quit: "Quit",
};

pub fn get_messages(lang: Language) -> &'static Messages {
    match lang {
        Language::Pt => &PT_MESSAGES,
        Language::En => &EN_MESSAGES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_for_all_languages_are_populated() {
        for lang in Language::ALL {
            let tr = get_messages(lang);
            assert!(!tr.app_title.is_empty());
            assert!(!tr.tab_buttons.is_empty());
            assert!(!tr.tab_motion.is_empty());
            assert!(!tr.tab_settings.is_empty());
            assert!(!tr.tab_raw_data.is_empty());
            assert!(!tr.save_settings.is_empty());
            assert!(!tr.sec_language.is_empty());
            assert!(!tr.tray_open_panel.is_empty());
            assert!(!tr.tray_quit.is_empty());
        }
    }

    #[test]
    fn parameterized_messages_formatting() {
        let tr_pt = get_messages(Language::Pt);
        let tr_en = get_messages(Language::En);

        assert!(tr_pt.tooltip_connected("COM3", 111.0).contains("COM3"));
        assert!(tr_pt.tooltip_connected("COM3", 111.0).contains("111"));
        assert!(tr_en.tooltip_connected("COM3", 111.0).contains("COM3"));

        assert_eq!(tr_pt.tooltip_disconnected(Language::Pt), "Desconectado (Aguardando receptor CP2102...)");
        assert_eq!(tr_en.tooltip_disconnected(Language::En), "Disconnected (Waiting for CP2102 receiver...)");

        let rec_pt = tr_pt.recording_active_msg(100, 1.5, Language::Pt);
        assert!(rec_pt.contains("100"));
        assert!(rec_pt.contains("1.5s"));

        let rec_en = tr_en.recording_active_msg(100, 1.5, Language::En);
        assert!(rec_en.contains("Recording"));
        assert!(rec_en.contains("100"));
    }
}
