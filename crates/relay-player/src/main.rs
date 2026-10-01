// A window, not a console, when double-clicked.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod win;

fn main() {
    #[cfg(windows)]
    std::process::exit(win::main());
    #[cfg(not(windows))]
    {
        eprintln!("The Relay player runs on Windows only.");
        std::process::exit(1);
    }
}
