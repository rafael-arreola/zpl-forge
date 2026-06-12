use std::collections::HashMap;

use zpl_forge::forge::pdf_native::PdfNativeBackend;
use zpl_forge::forge::png::PngBackend;
use zpl_forge::{Resolution, Unit, ZplEngine};

fn main() {
    let zpl = r#"^XA
        ^FX Top section with color logo and text.
        ^GLC#FF5733
        ^FO50,50^GB100,100,100^FS
        ^FO75,75^FR^GB100,100,100^FS

        ^GTC#2E86C1
        ^CF0,60
        ^FO220,50^FDIntershipping, Inc.^FS
        ^XZ"#;

    let engine = ZplEngine::new(
        zpl,
        Unit::Inches(4.0),
        Unit::Inches(2.0),
        Resolution::Dpi203,
    )
    .unwrap();

    let png = engine.render(PngBackend::new(), &HashMap::new()).unwrap();
    std::fs::write("/tmp/repro_fr.png", png).unwrap();

    let pdf = engine
        .render(PdfNativeBackend::new(), &HashMap::new())
        .unwrap();
    std::fs::write("/tmp/repro_fr.pdf", pdf).unwrap();

    println!("wrote /tmp/repro_fr.png and /tmp/repro_fr.pdf");
}
