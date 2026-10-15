//! Application entry point for LG Magic Motion user-mode driver.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use magic_motion::domain::config::Config;
use magic_motion::infra::driver::DriverHandle;
use magic_motion::presentation::run_gui_app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let headless = args.iter().any(|a| a == "--headless" || a == "--no-gui");

    let config = Config::load_or_default();

    if headless {
        println!("LG Magic Motion Driver iniciando em modo headless (sem interface gráfica)...");
        let mut driver = DriverHandle::start(config);

        let running = Arc::new(AtomicBool::new(true));
        let r = Arc::clone(&running);
        ctrlc_handler(move || {
            r.store(false, Ordering::SeqCst);
        });

        println!("Driver ativo! Pressione Ctrl+C para encerrar.");
        while running.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(100));
        }

        println!("Encerrando driver...");
        driver.stop();
        Ok(())
    } else {
        let driver = DriverHandle::start(config.clone());
        run_gui_app(driver, config)
    }
}

fn ctrlc_handler<F: Fn() + Send + 'static>(handler: F) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
        unsafe extern "system" fn ctrl_handler(_type: u32) -> i32 {
            1
        }
        let _ = SetConsoleCtrlHandler(Some(ctrl_handler), 1);
    }
    let _ = handler;
}
