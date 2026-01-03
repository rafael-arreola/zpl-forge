use zpl_forge::{Resolution, Unit, ZplEngine};

#[test]
fn test_missing_mandatory_font_name_a() {
    let input = "^XA\n^A\n^XZ";
    let result = ZplEngine::new(
        input,
        Unit::Inches(4.0),
        Unit::Inches(6.0),
        Resolution::Dpi203,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("line 2"));
}

#[test]
fn test_missing_mandatory_font_name_cf() {
    let input = "^XA\n^CF\n^XZ";
    let result = ZplEngine::new(
        input,
        Unit::Inches(4.0),
        Unit::Inches(6.0),
        Resolution::Dpi203,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("line 2"));
}

#[test]
fn test_invalid_graphic_box_params() {
    // ^GB requires width and height. Here we only provide width or invalid format.
    let input = "^XA\n^GB100\n^XZ";
    let result = ZplEngine::new(
        input,
        Unit::Inches(4.0),
        Unit::Inches(6.0),
        Resolution::Dpi203,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("line 2"));
}

#[test]
fn test_invalid_coordinates_fo() {
    let input = "^XA\n^FOA,10\n^XZ";
    let result = ZplEngine::new(
        input,
        Unit::Inches(4.0),
        Unit::Inches(6.0),
        Resolution::Dpi203,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("line 2"));
}

#[test]
fn test_multiple_lines_error_reporting() {
    let input = "^XA\n^FO100,100\n^A0\n^FDHello\n^FS\n^GB200\n^XZ";
    let result = ZplEngine::new(
        input,
        Unit::Inches(4.0),
        Unit::Inches(6.0),
        Resolution::Dpi203,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    // ^GB200 is missing the height parameter at line 6
    assert!(err.contains("line 6"));
}

#[test]
fn test_invalid_u32_parameter() {
    let input = "^XA\n^LLABC\n^XZ";
    let result = ZplEngine::new(
        input,
        Unit::Inches(4.0),
        Unit::Inches(6.0),
        Resolution::Dpi203,
    );
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("line 2"));
}
