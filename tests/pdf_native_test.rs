use std::collections::HashMap;
use zpl_forge::forge::pdf_native::PdfNativeBackend;
use zpl_forge::{Resolution, Unit, ZplEngine};

#[test]
fn test_01_native_pdf() {
    let zpl_input = r#"
    ^XA

    ^FX Top section with logo, name and address.
    ^CF0,60
    ^FO50,50^GB100,100,100^FS
    ^FO75,75^FR^GB100,100,100^FS
    ^FO93,93^GB40,40,40^FS
    ^FO220,50^FDIntershipping, Inc.^FS
    ^CF0,30
    ^FO220,115^FD1000 Shipping Lane^FS
    ^FO220,155^FDShelbyville TN 38102^FS
    ^FO220,195^FDUnited States (USA)^FS
    ^FO50,250^GB700,3,3^FS

    ^FX Second section with recipient address and permit information.
    ^CFA,30
    ^FO50,300^FDJohn Doe^FS
    ^FO50,340^FD100 Main Street^FS
    ^FO50,380^FDSpringfield TN 39021^FS
    ^FO50,420^FDUnited States (USA)^FS
    ^CFA,15
    ^FO600,300^GB150,150,3^FS
    ^FO638,340^FDPermit^FS
    ^FO638,390^FD123456^FS
    ^FO50,500^GB700,3,3^FS

    ^FX Third section with bar code.
    ^BY5,2,270
    ^FO100,550^BC^FD12345678^FS

    ^FX Fourth section (the two boxes on the bottom).
    ^FO50,900^GB700,250,3^FS
    ^FO400,900^GB3,250,3^FS
    ^CF0,40
    ^FO100,960^FDCtr. X34B-1^FS
    ^FO100,1010^FDREF1 F00B47^FS
    ^FO100,1060^FDREF2 BL4H8^FS
    ^CF0,190
    ^FO470,955^FDCA^FS

    ^XZ
"#;

    let engine = ZplEngine::new(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(6.0),
        Resolution::Dpi203,
    )
    .expect("Failed to parse ZPL");

    let backend = PdfNativeBackend::new();
    let pdf_bytes = engine
        .render(backend, &HashMap::new())
        .expect("Failed to render");

    std::fs::create_dir_all("examples").ok();
    std::fs::write("examples/test_01_native.pdf", &pdf_bytes).expect("Failed to write PDF");

    assert!(!pdf_bytes.is_empty(), "PDF output should not be empty");
    assert!(
        pdf_bytes.starts_with(b"%PDF"),
        "Output should be a valid PDF"
    );
}

#[test]
fn test_02_native_pdf() {
    let zpl_input = r#"
    ^XA
    ^FO5,5
    ^GB396,192,2,B,2
    ^FS

    ^FO5,150
    ^GB396,2,2
    ^FS

    ^FO198,150
    ^GB2,45,2
    ^FS

    ^FO270,5
    ^GB2,145,2
    ^FS

    ^FO270,80
    ^GB130,2,2
    ^FS

    ^FO20,14
    ^CFA,15
    ^FDRUTA
    ^FS

    ^FO15,45
    ^CFA,45
    ^FDB7-1
    ^FS

    ^FO20,115
    ^CFA,25
    ^FDKUW-068
    ^FS

    ^FO190,14
    ^CFA,15
    ^FDN/A
    ^FS

    ^FO20,166
    ^CFA,15
    ^FDDT:
    ^FS

    ^FO60,166
    ^CFA,14
    ^FD04/12/2025
    ^FS

    ^FO205,166
    ^CFA,15
    ^FDID:
    ^FS

    ^FO225,166
    ^CFA,12
    ^FD1496362371
    ^FS

    ^FO280,18
    ^CFA,15
    ^FDPARADA
    ^FS

    ^FO280,50
    ^CFA,15
    ^FD1
    ^FS

    ^FO280,90
    ^CFA,15
    ^FDPUNTO
    ^FS

    ^FO280,115
    ^GB30,30,2,,8
    ^FS

    ^FO289,121
    ^CFA,16
    ^FD0
    ^FS

    ^XZ
"#;

    let engine = ZplEngine::new(
        zpl_input,
        Unit::Inches(2.0),
        Unit::Inches(1.0),
        Resolution::Dpi203,
    )
    .expect("Failed to parse ZPL");

    let backend = PdfNativeBackend::new();
    let pdf_bytes = engine
        .render(backend, &HashMap::new())
        .expect("Failed to render");

    std::fs::create_dir_all("examples").ok();
    std::fs::write("examples/test_02_native.pdf", &pdf_bytes).expect("Failed to write PDF");

    assert!(!pdf_bytes.is_empty(), "PDF output should not be empty");
    assert!(
        pdf_bytes.starts_with(b"%PDF"),
        "Output should be a valid PDF"
    );
}
