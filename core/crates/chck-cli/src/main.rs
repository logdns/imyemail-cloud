fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("desktop-stdio") {
        if args.len() != 2 || chck_cli::desktop::run(&args[1]).is_err() {
            eprintln!("desktop_engine_unavailable");
            std::process::exit(1);
        }
        return;
    }
    match chck_cli::run(&args) {
        Ok(out) => println!("{out}"),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
