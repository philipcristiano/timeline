use std::path::Path;

fn main() {
    if std::env::var_os("TIMELINE_SKIP_TAILWIND").is_none() {
        build_tailwind();
    }
}

pub fn build_tailwind() {
    let target = std::env::var_os("OUT_DIR").unwrap();
    let target_dir = Path::new(&target).parent().unwrap().parent().unwrap();
    println!("Building");
    std::process::Command::new("tailwindcss")
        .arg("--content")
        .arg(format!(
            "{}/**/*.rs,./src/**/*.{{html,rs}},./index.html",
            target_dir.display(),
        ))
        .arg("-i")
        .arg("./tailwind/input.css")
        .arg("-o")
        .arg("./tailwind/tailwind.css")
        .arg("--minify")
        .output()
        .expect("failed to execute process");

    println!("cargo:rerun-if-changed=./src");
}
