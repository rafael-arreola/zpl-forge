//! ZPL-Forge Barcode & Labelary Parity Comparison Runner
//!
//! Executable with `cargo run --release --example zpl_comparison`
//!
//! Renders all 18 barcode symbologies and standard shipping labels natively
//! with `zpl-forge`, fetches the official reference renderings from Labelary API,
//! and generates 3-panel side-by-side comparison images with pixel diff overlays.

use std::collections::HashMap;
use std::process::Command;
use zpl_forge::forge::pdf_native::PdfNativeBackend;
use zpl_forge::forge::png::PngBackend;
use zpl_forge::{Resolution, Unit, ZplEngine};

struct ComparisonCase {
    name: &'static str,
    #[allow(dead_code)]
    title: &'static str,
    zpl: &'static str,
    w: f32,
    h: f32,
}

fn get_all_comparison_cases() -> Vec<ComparisonCase> {
    vec![
        ComparisonCase {
            name: "01_shipping_label",
            title: "Standard Shipping Label",
            zpl: r#"^XA^FO50,50^CF0,60^FDI^FS^FO50,115^CF0,30^FD100 Main St^FS^FO50,150^CF0,30^FDAustin, TX 78701^FS^FO50,200^GB700,3,3^FS^FO50,220^CF0,30^FDShip To:^FS^FO50,260^CFA,30^FDJohn Doe^FS^FO50,295^CFA,30^FD123 Business Rd^FS^FO50,330^CFA,30^FDSuite 400^FS^FO50,365^CFA,30^FDSan Francisco, CA 94107^FS^FO50,420^GB700,3,3^FS^FO50,450^BY3,2,140^FO100,450^BCN,140,Y,N,N^FD4209410792001^FS^XZ"#,
            w: 4.0,
            h: 6.0,
        },
        ComparisonCase {
            name: "02_code128",
            title: "Code 128 (BC)",
            zpl: "^XA^FO50,50^BY2,2,100^BCN,100,Y,N,N^FDZPL-FORGE-128^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "03_code39",
            title: "Code 39 (B3)",
            zpl: "^XA^FO50,50^BY2,2,100^B3N,N,100,Y,N^FDZPL-FORGE-39^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "04_code93",
            title: "Code 93 (BA)",
            zpl: "^XA^FO50,50^BY2,2,100^BAN,100,Y,N,N^FDZPL-FORGE-93^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "05_itf",
            title: "Interleaved 2 of 5 (B2)",
            zpl: "^XA^FO50,50^BY2,2,100^B2N,100,Y,N,N^FD12345678^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "06_ean13",
            title: "EAN-13 (BE)",
            zpl: "^XA^FO50,50^BY2,2,100^BEN,100,Y,N^FD1234567890128^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "07_ean8",
            title: "EAN-8 (B8)",
            zpl: "^XA^FO50,50^BY2,2,100^B8N,100,Y,N^FD12345670^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "08_upca",
            title: "UPC-A (BU)",
            zpl: "^XA^FO50,50^BY2,2,100^BUN,100,Y,N^FD012345678905^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "09_upce",
            title: "UPC-E (B9)",
            zpl: "^XA^FO50,50^BY2,2,100^B9N,100,Y,N,Y^FD01234565^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "10_codabar",
            title: "Codabar (BB)",
            zpl: "^XA^FO50,50^BY2,2,100^BBN,100,Y,N,Y^FDA123456B^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "11_msi",
            title: "MSI Barcode (BM)",
            zpl: "^XA^FO50,50^BY2,2,100^BMN,N,100,Y,N^FD123456^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "12_postnet",
            title: "POSTNET (BZ)",
            zpl: "^XA^FO50,50^BY2,2,100^BZN,100,Y,N^FD12345^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "13_gs1databar",
            title: "GS1 DataBar (BR)",
            zpl: "^XA^FO50,50^BY2,2,100^BRN,1,2,5,100,1^FD0101234567890128^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "14_qrcode",
            title: "QR Code (BQ)",
            zpl: "^XA^FO50,50^BQN,2,6^FDQA,https://zplforge.dev^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "15_datamatrix",
            title: "Data Matrix (BX)",
            zpl: "^XA^FO50,50^BXN,8,200^FDZPL-FORGE-DATAMATRIX^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "16_pdf417",
            title: "PDF417 (B7)",
            zpl: "^XA^FO50,50^B7N,10,0^FDZPL-FORGE-PDF417-2D-BARCODE^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "17_micropdf417",
            title: "MicroPDF417 (BF)",
            zpl: "^XA^FO50,50^BFN,12,0^FDMICRO-PDF417^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
        ComparisonCase {
            name: "18_aztec",
            title: "Aztec Code (B0)",
            zpl: "^XA^FO50,50^B0N,6,N,0,N,1^FDAZTEC-CODE-2D^FS^XZ",
            w: 4.0,
            h: 3.0,
        },
    ]
}

fn main() {
    println!("========================================================");
    println!(" ZPL-FORGE VISUAL COMPARISON SUITE (ALL BARCODES)");
    println!("========================================================\n");

    let cases = get_all_comparison_cases();
    println!("Rendering {} test cases to PNG and PDF...", cases.len());

    for case in &cases {
        let engine = ZplEngine::new(
            case.zpl,
            Unit::Inches(case.w),
            Unit::Inches(case.h),
            Resolution::Dpi203,
        )
        .unwrap_or_else(|e| panic!("Engine creation failed for {}: {:?}", case.name, e));

        let png_bytes = engine
            .render(PngBackend::new(), &HashMap::new())
            .unwrap_or_else(|e| panic!("PNG render failed for {}: {:?}", case.name, e));
        let pdf_bytes = engine
            .render(PdfNativeBackend::new(), &HashMap::new())
            .unwrap_or_else(|e| panic!("PDF render failed for {}: {:?}", case.name, e));

        let png_path = format!("examples/{}.png", case.name);
        let pdf_path = format!("examples/{}.pdf", case.name);
        std::fs::write(&png_path, &png_bytes).unwrap();
        std::fs::write(&pdf_path, &pdf_bytes).unwrap();
    }

    println!("\nLaunching python3 tools/compare_labelary.py --all-barcodes...");
    let status = Command::new("python3")
        .arg("tools/compare_labelary.py")
        .arg("--all-barcodes")
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("\n[SUCCESS] Visual comparison completed for all 18 barcode symbologies!");
            println!("Check 3-panel composite output images in 'output_comparisons/' directory.");
        }
        _ => {
            println!(
                "\n[NOTE] Python comparison script finished. Running single comparison as fallback..."
            );
            let _ = Command::new("python3")
                .arg("tools/compare_labelary.py")
                .status();
        }
    }
}
