//! Writes the Staunton chess pack into the folder named on the command line.

fn main() {
    let Some(out) = std::env::args_os().nth(1) else {
        eprintln!("usage: make-staunton-pack <folder>");
        std::process::exit(2);
    };
    match pack_staunton_chess::write(std::path::Path::new(&out)) {
        Ok(pack) => println!("wrote {} files of {}", pack.items.len(), pack.name),
        Err(e) => {
            eprintln!("could not write the pack: {e}");
            std::process::exit(1);
        }
    }
}
