fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match chck_cli::run(&args) {
        Ok(out) => println!("{out}"),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
