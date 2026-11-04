use std::env;
use winresource::WindowsResource;

fn main() -> anyhow::Result<()> {
    // 1. Cargo sagen, dass das Skript NUR bei Änderungen am Icon oder der UI laufen soll
    println!("cargo:rerun-if-changed=assets/icons/review_helper.ico");
    println!("cargo:rerun-if-changed=ui"); // Überwacht den gesamten ui-Ordner

    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new().set_icon("assets/icons/review_helper.ico").compile()?;
    }

    let config = slint_build::CompilerConfiguration::new().with_style("fluent-dark".into());
    slint_build::compile_with_config("ui/appwindow.slint", config)?;
    Ok(())
}
