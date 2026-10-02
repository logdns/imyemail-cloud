//! Linux 客户端。`--features gtk` 时默认启动 libadwaita 壳；否则复用 imyemail-cloud CLI。

#![cfg_attr(not(feature = "gtk"), allow(dead_code))]

mod content_policy;
mod engine_bridge;
mod i18n;
mod layout;
mod markdown;
mod secrets;
mod util;

#[cfg(feature = "gtk")]
mod app;
#[cfg(feature = "gtk")]
mod pages;
#[cfg(feature = "gtk")]
mod task;
#[cfg(feature = "gtk")]
mod widgets;
#[cfg(feature = "gtk")]
mod window;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if should_gui(&args) {
        #[cfg(feature = "gtk")]
        {
            crate::app::run();
            return;
        }
        #[cfg(not(feature = "gtk"))]
        {
            match chck_cli::run(&["ui".into()]) {
                Ok(out) => {
                    println!("{out}");
                    println!("GTK4 + libadwaita：cargo run --features gtk");
                    return;
                }
                Err(err) => {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            }
        }
    }
    match chck_cli::run(&args) {
        Ok(out) => println!("{out}"),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

fn should_gui(args: &[String]) -> bool {
    if args.iter().any(|a| a == "--help" || a == "-h" || a == "help") {
        return false;
    }
    if args.first().map(String::as_str) == Some("gui") || args.iter().any(|a| a == "--gui") {
        return true;
    }
    if args.iter().any(|a| a.starts_with("mailto:")) {
        return true;
    }
    args.is_empty() && cfg!(feature = "gtk")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_flag() {
        assert!(should_gui(&["gui".into()]));
        assert!(should_gui(&["--gui".into()]));
        assert!(should_gui(&["mailto:a@b.c".into()]));
        assert!(!should_gui(&["help".into()]));
        assert!(!should_gui(&["accounts".into()]));
    }
}
