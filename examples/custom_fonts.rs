use std::collections::HashMap;
use std::sync::Arc;
use zpl_forge::forge::png::PngBackend;
use zpl_forge::{FontManager, Resolution, Unit, ZplEngine};

fn main() -> zpl_forge::ZplResult<()> {
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
                println!("==========================================================");
                println!("Missing font file: {}", path);
                println!("To run this example, please download the open-source fonts");
                println!("by executing the script: examples/fonts/download_fonts.sh");
                println!("==========================================================");
                return Ok(());
            }
        };
        font_manager.register_font(name, &font_bytes, *id, *id)?;
    }

    let zpl_input = "^XA\n\
        ^FO50,50^AAN,50,50^FDThis is Abril Fatface (Identifier A)^FS\n\
        ^FO50,120^ABN,50,50^FDThis is Anton (Identifier B)^FS\n\
        ^FO50,190^ACN,50,50^FDThis is Bebas Neue (Identifier C)^FS\n\
        ^FO50,260^ADN,50,50^FDThis is Inconsolata (Identifier D)^FS\n\
        ^FO50,330^AEN,50,50^FDThis is Lato (Identifier E)^FS\n\
        ^FO50,400^AFN,50,50^FDThis is Lobster (Identifier F)^FS\n\
        ^FO50,470^AGN,50,50^FDThis is Montserrat (Identifier G)^FS\n\
        ^FO50,540^AHN,50,50^FDThis is Open Sans (Identifier H)^FS\n\
        ^FO50,610^AIN,50,50^FDThis is Pacifico (Identifier I)^FS\n\
        ^FO50,680^AJN,50,50^FDThis is Ubuntu (Identifier J)^FS\n\
        ^XZ";

    let mut engine = ZplEngine::new(
        zpl_input,
        Unit::Inches(6.0),
        Unit::Inches(4.0),
        Resolution::Dpi203,
    )?;

    engine.set_fonts(Arc::new(font_manager));

    println!("Rendering ZPL to PNG...");
    let png_backend = PngBackend::new();
    let png_bytes = engine.render(png_backend, &HashMap::new())?;

    std::fs::write("examples/custom_fonts_output.png", png_bytes)
        .expect("Failed to write output image");
    println!("Successfully rendered to examples/custom_fonts_output.png");

    Ok(())
}
