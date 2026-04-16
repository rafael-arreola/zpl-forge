use std::collections::HashMap;
use std::sync::Arc;
use zpl_forge::forge::png::PngBackend;
use zpl_forge::{FontManager, Resolution, Unit, ZplEngine};

#[test]
fn test_multiple_custom_fonts() {
    let mut font_manager = FontManager::default();
    let font_dir = "examples/fonts/";

    let fonts_to_load = [
        ("AbrilFatface", "AbrilFatface.ttf", 'A'),
        ("Anton", "Anton.ttf", 'B'),
        ("BebasNeue", "BebasNeue.ttf", 'C'),
        ("Inconsolata", "Inconsolata.ttf", 'D'),
        ("Lato", "Lato.ttf", 'E'),
        ("Lobster", "Lobster.ttf", 'F'),
        ("Montserrat", "Montserrat.ttf", 'G'),
        ("OpenSans", "OpenSans.ttf", 'H'),
        ("Pacifico", "Pacifico.ttf", 'I'),
        ("Ubuntu", "Ubuntu.ttf", 'J'),
    ];

    for (name, filename, id) in fonts_to_load.iter() {
        let path = format!("{}{}", font_dir, filename);
        let font_bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                println!(
                    "Skipping test: Font file {} not found. Run examples/fonts/download_fonts.sh first.",
                    path
                );
                return;
            }
        };

        font_manager
            .register_font(name, &font_bytes, *id, *id)
            .unwrap_or_else(|_| panic!("Failed to register font: {}", name));
    }

    let zpl_input = "^XA\n\
        ^FO10,10^AAN,30,30^FDAbrilFatface Font Test^FS\n\
        ^FO10,50^ABN,30,30^FDAnton Font Test^FS\n\
        ^FO10,90^ACN,30,30^FDBebasNeue Font Test^FS\n\
        ^FO10,130^ADN,30,30^FDInconsolata Font Test^FS\n\
        ^FO10,170^AEN,30,30^FDLato Font Test^FS\n\
        ^FO10,210^AFN,30,30^FDLobster Font Test^FS\n\
        ^FO10,250^AGN,30,30^FDMontserrat Font Test^FS\n\
        ^FO10,290^AHN,30,30^FDOpenSans Font Test^FS\n\
        ^FO10,330^AIN,30,30^FDPacifico Font Test^FS\n\
        ^FO10,370^AJN,30,30^FDUbuntu Font Test^FS\n\
        ^XZ";

    let mut engine = ZplEngine::new(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(4.0),
        Resolution::Dpi203,
    )
    .expect("Failed to parse ZPL with multiple fonts");

    engine.set_fonts(Arc::new(font_manager));

    let png_backend = PngBackend::new();
    let result = engine.render(png_backend, &HashMap::new());

    assert!(
        result.is_ok(),
        "Failed to render ZPL containing multiple custom fonts"
    );

    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "Rendered PNG bytes are empty");
}
