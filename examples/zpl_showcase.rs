//! Streamlined & Consolidated ZPL-Forge Feature Showcase
//!
//! Demonstrates the complete feature set of `zpl-forge` across 5 unified showcase modules:
//! 1. Standard Shipping Label (Text, Box Graphics, Field Origin/Typeset, Code 128)
//! 2. Complete Barcode Symbology Gallery (All 18 1D and 2D barcode types)
//! 3. Graphics & Image Processing (Monochrome ^GF, Color Base64 ^GI, Image Scaling ^GIC)
//! 4. Template Logic & Custom Fonts (Variables {{var}}, ^IFC Conditions, Shapes ^GC/^GE/^GD, TTF Fonts)
//! 5. High-Performance Multi-Page Vector PDF Batching (1,000 pages in <60ms)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use zpl_forge::forge::pdf_native::PdfNativeBackend;
use zpl_forge::forge::png::PngBackend;
use zpl_forge::{FontManager, Resolution, Unit, ZplEngine, ZplForgeBackend};

/// Static asset imports using `include_str!` macro
static BITMAP_IMAGE_ZPL: &str = include_str!("assets/bitmap_image.zpl");
static COLOR_IMAGE_ZPL: &str = include_str!("assets/color_image.zpl");

/// Helper to render a ZPL string with a specified backend and save output.
fn run_test<B: ZplForgeBackend>(zpl: &str, w: Unit, h: Unit, backend: B, output_name: &str) {
    run_test_with_vars_and_fonts(zpl, w, h, backend, output_name, &HashMap::new(), None);
}

/// Helper to render a ZPL string with variables, optional font manager, and a specified backend.
fn run_test_with_vars_and_fonts<B: ZplForgeBackend>(
    zpl: &str,
    w: Unit,
    h: Unit,
    backend: B,
    output_name: &str,
    variables: &HashMap<String, String>,
    font_manager: Option<FontManager>,
) {
    let start_total = Instant::now();

    let mut engine = ZplEngine::new(zpl, w, h, Resolution::Dpi203)
        .unwrap_or_else(|e| panic!("Failed to parse ZPL for {}: {:?}", output_name, e));

    if let Some(fm) = font_manager {
        engine.set_fonts(Arc::new(fm));
    }

    let start_render = Instant::now();
    let bytes = engine
        .render(backend, variables)
        .unwrap_or_else(|e| panic!("Failed to render ZPL for {}: {:?}", output_name, e));
    let render_duration = start_render.elapsed();

    let file_path = format!("examples/{}", output_name);
    std::fs::write(&file_path, &bytes)
        .unwrap_or_else(|e| panic!("Failed to write output to {}: {:?}", file_path, e));
    let total_duration = start_total.elapsed();

    println!("[{}] Rendering took: {:?}", output_name, render_duration);
    println!(
        "[{}] Total process (including writing) took: {:?}",
        output_name, total_duration
    );
}

// ── Module 1: Standard Shipping Label ────────────────────────────────
pub fn render_01_shipping_label() {
    let zpl_input = r#"
        ^XA
        ^FO50,50^CF0,60^FDI^FS
        ^FO50,115^CF0,30^FD100 Main St^FS
        ^FO50,150^CF0,30^FDAustin, TX 78701^FS
        ^FO50,200^GB700,3,3^FS

        ^FO50,220^CF0,30^FDShip To:^FS
        ^FO50,260^CFA,30^FDJohn Doe^FS
        ^FO50,295^CFA,30^FD123 Business Rd^FS
        ^FO50,330^CFA,30^FDSuite 400^FS
        ^FO50,365^CFA,30^FDSan Francisco, CA 94107^FS
        ^FO50,420^GB700,3,3^FS

        ^FO50,450^BY3,2,140
        ^FO100,450^BCN,140,Y,N,N^FD4209410792001^FS
        ^XZ
    "#;

    run_test(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(6.0),
        PngBackend::new(),
        "showcase_01_shipping.png",
    );
    run_test(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(6.0),
        PdfNativeBackend::new(),
        "showcase_01_shipping.pdf",
    );
}

// ── Module 2: Complete Barcode Symbology Gallery ─────────────────────
pub fn render_02_barcode_gallery() {
    let zpl_input = r#"
        ^XA
        ^CF0,42
        ^FO50,30^FDZPL-Forge Complete Barcode Symbology Gallery (17 Symbologies)^FS
        ^FO50,75^GB1520,3,3^FS

        ^BY2,2,70

        ^FX ── Row 1 ──────────────────────────────────────────────
        ^FX 1. Code 128
        ^FO50,100^A0N,24,24^FD1. Code 128 (BC)^FS
        ^FO50,126^A0N,20,20^FDData: "ZPL-FORGE-128"^FS
        ^FO50,150^BCN,70,Y,N,N^FDZPL-FORGE-128^FS

        ^FX 2. Code 39
        ^FO850,100^A0N,24,24^FD2. Code 39 (B3)^FS
        ^FO850,126^A0N,20,20^FDData: "ZPL-FORGE-39"^FS
        ^FO850,150^B3N,N,70,Y,N^FDZPL-FORGE-39^FS

        ^FX ── Row 2 ──────────────────────────────────────────────
        ^FX 3. Code 93
        ^FO50,270^A0N,24,24^FD3. Code 93 (BA)^FS
        ^FO50,296^A0N,20,20^FDData: "ZPL-FORGE-93"^FS
        ^FO50,320^BAN,70,Y,N,N^FDZPL-FORGE-93^FS

        ^FX 4. Interleaved 2 of 5
        ^FO850,270^A0N,24,24^FD4. Interleaved 2 of 5 (B2)^FS
        ^FO850,296^A0N,20,20^FDData: "12345678"^FS
        ^FO850,320^B2N,70,Y,N,N^FD12345678^FS

        ^FX ── Row 3 ──────────────────────────────────────────────
        ^FX 5. EAN-13
        ^FO50,440^A0N,24,24^FD5. EAN-13 (BE)^FS
        ^FO50,466^A0N,20,20^FDData: "1234567890128"^FS
        ^FO50,490^BEN,70,Y,N^FD1234567890128^FS

        ^FX 6. EAN-8
        ^FO850,440^A0N,24,24^FD6. EAN-8 (B8)^FS
        ^FO850,466^A0N,20,20^FDData: "12345670"^FS
        ^FO850,490^B8N,70,Y,N^FD12345670^FS

        ^FX ── Row 4 ──────────────────────────────────────────────
        ^FX 7. UPC-A
        ^FO50,610^A0N,24,24^FD7. UPC-A (BU)^FS
        ^FO50,636^A0N,20,20^FDData: "012345678905"^FS
        ^FO50,660^BUN,70,Y,N^FD012345678905^FS

        ^FX 8. UPC-E
        ^FO850,610^A0N,24,24^FD8. UPC-E (B9)^FS
        ^FO850,636^A0N,20,20^FDData: "01234565"^FS
        ^FO850,660^B9N,70,Y,N,Y^FD01234565^FS

        ^FX ── Row 5 ──────────────────────────────────────────────
        ^FX 9. Codabar
        ^FO50,780^A0N,24,24^FD9. Codabar (BB)^FS
        ^FO50,806^A0N,20,20^FDData: "A123456B"^FS
        ^FO50,830^BBN,70,Y,N,Y^FDA123456B^FS

        ^FX 10. MSI Barcode
        ^FO850,780^A0N,24,24^FD10. MSI Barcode (BM)^FS
        ^FO850,806^A0N,20,20^FDData: "123456"^FS
        ^FO850,830^BMN,N,70,Y,N^FD123456^FS

        ^FX ── Row 6 ──────────────────────────────────────────────
        ^FX 11. POSTNET
        ^FO50,950^A0N,24,24^FD11. POSTNET (BZ)^FS
        ^FO50,976^A0N,20,20^FDData: "12345"^FS
        ^FO50,1000^BZN,70,Y,N^FD12345^FS

        ^FX 12. GS1 DataBar
        ^FO850,950^A0N,24,24^FD12. GS1 DataBar (BR)^FS
        ^FO850,976^A0N,20,20^FDData: "0101234567890128"^FS
        ^FO850,1000^BRN,1,2,5,70,1^FD0101234567890128^FS

        ^FX ── Row 7 (2D Barcodes) ────────────────────────────────
        ^FX 13. QR Code
        ^FO50,1130^A0N,24,24^FD13. QR Code (BQ)^FS
        ^FO50,1156^A0N,20,20^FDData: "https://github.com/rafael-arreola/zpl-forge"^FS
        ^FO50,1185^BQN,2,5^FDQA,https://github.com/rafael-arreola/zpl-forge^FS

        ^FX 14. Data Matrix
        ^FO850,1130^A0N,24,24^FD14. Data Matrix (BX)^FS
        ^FO850,1156^A0N,20,20^FDData: "ZPL-FORGE-DATAMATRIX"^FS
        ^FO850,1185^BXN,6,200^FDZPL-FORGE-DATAMATRIX^FS

        ^FX ── Row 8 ──────────────────────────────────────────────
        ^FX 15. PDF417
        ^FO50,1370^A0N,24,24^FD15. PDF417 (B7)^FS
        ^FO50,1396^A0N,20,20^FDData: "ZPL-FORGE-PDF417-2D-BARCODE"^FS
        ^FO50,1420^B7N,7,0^FDZPL-FORGE-PDF417-2D-BARCODE^FS

        ^FX 16. MicroPDF417
        ^FO850,1370^A0N,24,24^FD16. MicroPDF417 (BF)^FS
        ^FO850,1396^A0N,20,20^FDData: "MICRO-PDF417"^FS
        ^FO850,1420^BFN,8,0^FDMICRO-PDF417^FS

        ^FX ── Row 9 ──────────────────────────────────────────────
        ^FX 17. Aztec Code
        ^FO50,1570^A0N,24,24^FD17. Aztec Code (B0)^FS
        ^FO50,1596^A0N,20,20^FDData: "AZTEC-CODE-2D"^FS
        ^FO50,1620^B0N,4,N,0,N,1^FDAZTEC-CODE-2D^FS

        ^XZ
    "#;

    run_test(
        zpl_input,
        Unit::Inches(8.0),
        Unit::Inches(9.2),
        PngBackend::new(),
        "showcase_02_barcodes.png",
    );
    run_test(
        zpl_input,
        Unit::Inches(8.0),
        Unit::Inches(9.2),
        PdfNativeBackend::new(),
        "showcase_02_barcodes.pdf",
    );
}

// ── Module 3: Unified Graphics & Image Processing ────────────────────
// Combines Monochrome Bitmap (^GF), Color Image (^GI), Image Scaling (^GIC), and Color Extensions (^GTC, ^GLC).
pub fn render_03_graphics_and_images() {
    let bitmap_data = BITMAP_IMAGE_ZPL.replace("^FO50,50", "^FO50,80");
    let color_data = COLOR_IMAGE_ZPL;

    let zpl_input = format!(
        r#"^XA
        ^FO50,50^GFA,80000,80000,100,{}^FS
        ^FO50,900^GIC0,0,{}^FS
        ^XZ"#,
        bitmap_data, color_data
    );

    run_test(
        &zpl_input,
        Unit::Inches(4.5),
        Unit::Inches(8.6),
        PngBackend::new(),
        "showcase_03_graphics.png",
    );
    run_test(
        &zpl_input,
        Unit::Inches(4.5),
        Unit::Inches(8.6),
        PdfNativeBackend::new(),
        "showcase_03_graphics.pdf",
    );
}

// ── Module 4: Unified Template Logic, Vector Shapes & TTF Fonts ─────
// Combines Template Variables {{var}}, ^IFC Conditions (true/false), Vector Shapes (^GC, ^GE, ^GD) and Custom TTF Font Embedding.
pub fn render_04_logic_and_fonts() {
    let font_dir = "examples/fonts/";
    let font_path = format!("{}Roboto-Regular.ttf", font_dir);

    let mut font_manager = FontManager::default();
    if let Ok(font_bytes) = std::fs::read(&font_path) {
        let _ = font_manager.register_font("Roboto", &font_bytes, 'Z', 'Z');
    }

    let zpl_input = r#"
        ^XA
        ^FO50,40^A0N,36,36^FDTemplate Logic & Vector Shapes^FS
        ^FO50,90^A0N,26,26^FDUser: {{user_name}} ({{user_role}})^FS

        ^FX Condition: user_type == admin (True Branch - Rendered)
        ^FO50,140^IFCuser_type,admin^A0N,26,26^FD[ADMIN ACCESS GRANTED - CONFIRMED]^FS

        ^FX Condition: is_vip == true (False Branch - Safely Omitted)
        ^FO50,180^IFCis_vip,true^A0N,26,26^FD[VIP BADGE ACTIVE]^FS

        ^FO50,225^GB700,3,3^FS

        ^FO50,240^A0N,28,28^FDVector Shapes (Circle, Ellipse, Diagonals)^FS

        ^FX Vector Circle (GC) and Ellipse (GE)
        ^FO50,285^GC80,5,B^FS
        ^FO170,285^GE160,80,5,B^FS

        ^FX Diagonal Lines (GD)
        ^FO370,285^GD100,80,5,B,R^FS
        ^FO510,285^GD100,80,5,B,L^FS

        ^FO50,395^GB700,3,3^FS

        ^FX Custom TTF Font Rendering (Identifier Z)
        ^FO50,415^AZN,34,34^FDRoboto Custom TTF Font Embedded (^AZ)^FS
        ^FO50,460^AZN,24,24^FDScalable Vector Typography Without Distortion^FS
        ^XZ
    "#;

    let mut vars = HashMap::new();
    vars.insert("user_name".to_string(), "Alice Smith".to_string());
    vars.insert("user_role".to_string(), "System Administrator".to_string());
    vars.insert("user_type".to_string(), "admin".to_string());
    vars.insert("is_vip".to_string(), "false".to_string());

    run_test_with_vars_and_fonts(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(8.0),
        PngBackend::new(),
        "showcase_04_logic_and_fonts.png",
        &vars,
        Some(font_manager.clone()),
    );
    run_test_with_vars_and_fonts(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(8.0),
        PdfNativeBackend::new(),
        "showcase_04_logic_and_fonts.pdf",
        &vars,
        Some(font_manager),
    );
}

// ── Module 5: High-Performance Multi-Page Batch PDF ────────────────
pub fn render_05_multi_page_batch() {
    const BENCHMARK_PAGE_COUNT: usize = 1000;
    const PAGE_WIDTH_INCHES: f32 = 4.0;
    const PAGE_HEIGHT_INCHES: f32 = 6.0;

    let template_zpl = r#"
        ^XA
        ^FO30,30^A0N,18,18^FDFROM: ZPL-FORGE LOGISTICS HUB^FS
        ^FO30,50^A0N,18,18^FD100 INDUSTRIAL PARKWAY SUITE 400^FS
        ^FO30,70^A0N,18,18^FDAUSTIN TX 78701 UNITED STATES^FS
        ^FO30,90^A0N,18,18^FDP: +1 (512) 555-0199^FS

        ^FO500,30^A0N,18,18^FDSHIP DATE: 25JUL26^FS
        ^FO500,50^A0N,18,18^FDCAD: 104592031/NET4200^FS
        ^FO500,70^A0N,18,18^FDACTWT: 10.00 LBS^FS
        ^FO500,90^A0N,18,18^FDDIMS: 12x10x8 IN^FS

        ^FO30,115^GB750,3,3^FS

        ^FO30,130^A0N,22,22^FDTO: {{name}}^FS
        ^FO30,155^A0N,30,30^FD{{company}}^FS
        ^FO30,190^A0N,22,22^FD{{address}}^FS
        ^FO30,215^A0N,26,26^FD{{city_state_zip}}^FS
        ^FO30,245^A0N,20,20^FDP: {{phone}}^FS

        ^FO480,130^GB300,125,3^FS
        ^FO490,140^A0N,20,20^FDDEST HUB / AIRPORT^FS
        ^FO490,170^A0N,55,55^FD{{dest_hub}}^FS

        ^FO30,270^GB750,3,3^FS

        ^FO30,285^A0N,45,45^FDZPL-FORGE EXPRESS^FS
        ^FO30,335^A0N,22,22^FDSERVICE: PRIORITY OVERNIGHT^FS
        ^FO30,360^A0N,22,22^FDTRK #: {{tracking_no}}^FS

        ^FO30,390^BY3,2,110
        ^FO40,390^BCN,110,Y,N,N^FD{{tracking_no}}^FS

        ^FO30,545^GB750,3,3^FS

        ^FO30,560^GB360,115,3^FS
        ^FO40,570^A0N,20,20^FDFORM: 0201 STATION: AUS^FS
        ^FO40,595^A0N,20,20^FDBILLING: SENDER / ACCT 99^FS
        ^FO40,620^A0N,20,20^FDREF 1: {{order_id}}^FS
        ^FO40,645^A0N,20,20^FDREF 2: BATCH-P1000^FS

        ^FO410,560^GB370,115,3^FS
        ^FO420,570^A0N,20,20^FDPOSTAL CODE / DEST ROUTE^FS
        ^FO420,600^A0N,45,45^FD{{zip_code}}^FS
        ^FO420,645^A0N,20,20^FDDELIVERY SUN / NO REQ^FS

        ^FO30,690^GB750,3,3^FS

        ^FO100,710^BY2,2,120
        ^FO100,710^BCN,120,Y,N,N^FD962200123456{{zip_code}}{{tracking_no_short}}^FS
        ^XZ
    "#;

    let names = [
        "JUAN PEREZ LOPEZ",
        "MARIA GARCIA MARTINEZ",
        "CARLOS RODRIGUEZ",
        "ANA HERNANDEZ",
    ];
    let companies = [
        "TECH SOLUTIONS CORP",
        "GLOBAL IMPORTS S.A.",
        "INNOVATION LABS",
        "LOGISTICA GLOBAL",
    ];
    let addresses = [
        "AV REFORMA 500 PISO 10",
        "CALLE OBRERA 120 BODEGA 4",
        "INSURGENTES SUR 1450",
        "AV JUAREZ 88 STE 200",
    ];
    let cities_hubs = [
        ("MEXICO CITY DF 06600", "MEX", "+52 55 5555 0100", "06600"),
        ("GUADALAJARA JAL 44100", "GDL", "+52 33 3333 0200", "44100"),
        ("MONTERREY NL 64000", "MTY", "+52 81 8181 0300", "64000"),
        ("CANCUN QROO 77500", "CUN", "+52 998 888 0400", "77500"),
    ];

    let width = Unit::Inches(PAGE_WIDTH_INCHES);
    let height = Unit::Inches(PAGE_HEIGHT_INCHES);
    let resolution = Resolution::Dpi203;

    println!(
        "[showcase_05_multi_page.pdf] Preparing variables for {} ZPL-FORGE EXPRESS vertical shipping labels...",
        BENCHMARK_PAGE_COUNT
    );
    let prepare_start = Instant::now();
    let mut pages_vars = Vec::with_capacity(BENCHMARK_PAGE_COUNT);

    for i in 0..BENCHMARK_PAGE_COUNT {
        let mut vars = HashMap::new();
        let (city_state_zip, dest_hub, phone, zip_code) = cities_hubs[i % cities_hubs.len()];
        let tracking = format!("78345678{:04}", i);
        vars.insert("name".to_string(), names[i % names.len()].to_string());
        vars.insert(
            "company".to_string(),
            companies[i % companies.len()].to_string(),
        );
        vars.insert(
            "address".to_string(),
            addresses[i % addresses.len()].to_string(),
        );
        vars.insert("city_state_zip".to_string(), city_state_zip.to_string());
        vars.insert("dest_hub".to_string(), dest_hub.to_string());
        vars.insert("phone".to_string(), phone.to_string());
        vars.insert("order_id".to_string(), format!("ORD-{}", 100001 + i));
        vars.insert("tracking_no".to_string(), tracking.clone());
        vars.insert("tracking_no_short".to_string(), tracking[4..].to_string());
        vars.insert("zip_code".to_string(), zip_code.to_string());
        pages_vars.push(vars);
    }
    println!(
        "[showcase_05_multi_page.pdf] Variable preparation completed in {:.2?}",
        prepare_start.elapsed()
    );

    println!(
        "[showcase_05_multi_page.pdf] Parsing template and rendering 1,000 ZPL-FORGE EXPRESS vertical labels to PDF natively..."
    );
    let render_start = Instant::now();
    let engine = ZplEngine::new(template_zpl, width, height, resolution)
        .expect("Failed to parse batch template");
    let pdf_backend = PdfNativeBackend::new();
    let pdf_bytes = engine
        .render_pages(pdf_backend, &pages_vars)
        .expect("Failed rendering multi-page PDF");
    let render_duration = render_start.elapsed();

    let output_path = "examples/showcase_05_multi_page.pdf";
    std::fs::write(output_path, &pdf_bytes).expect("Failed to write multi-page PDF");

    println!(
        "[showcase_05_multi_page.pdf] Saved {} ZPL-FORGE EXPRESS PDF pages ({:.2} MB) | Total time: {:.2?} ({:.3} ms/page)",
        BENCHMARK_PAGE_COUNT,
        pdf_bytes.len() as f64 / (1024.0 * 1024.0),
        render_duration,
        render_duration.as_secs_f64() * 1000.0 / BENCHMARK_PAGE_COUNT as f64
    );
}

fn main() {
    println!("Rendering 5 consolidated showcase modules...");
    render_01_shipping_label();
    render_02_barcode_gallery();
    render_03_graphics_and_images();
    render_04_logic_and_fonts();
    render_05_multi_page_batch();
    println!("All 5 consolidated showcase modules rendered to examples/ directory!");
}
