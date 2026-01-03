use std::collections::HashMap;
use zpl_forge::forge::png::PngBackend;
use zpl_forge::{Resolution, Unit, ZplEngine};

fn main() {
    let zpl_input = r#"
        ^XA
        ^FO50,50^A0N,50,50^FDZPL Forge^FS
        ^FO50,120^GB300,100,2^FS
        ^FO70,140^A0N,30,30^FDHello World!^FS
        ^FO50,250^BCN,100,Y,N,N^FD12345678^FS
        ^XZ
    "#;

    let engine = ZplEngine::new(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(3.0),
        Resolution::Dpi203,
    )
    .expect("Error parsing ZPL");

    let png_backend = PngBackend::new();
    let png_bytes = engine
        .render(png_backend, &HashMap::new())
        .expect("Error rendering");

    std::fs::write("example_output.png", png_bytes).expect("Error writing file");
    println!("Label successfully generated in 'example_output.png'");
}
