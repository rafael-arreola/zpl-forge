use std::fs;
use zpl_forge::tools::zpl_encode;

#[test]
fn test_zpl_encode_from_jpg_file() {
    // Load the test image from the tests directory
    let image_path = "tests/test.jpg";
    let image_bytes = fs::read(image_path)
        .expect("Failed to read tests/test.jpg. Ensure the file exists in the tests directory.");

    // Perform encoding
    let result = zpl_encode(&image_bytes);

    // Verify the result
    assert!(
        result.is_ok(),
        "Encoding process failed: {:?}",
        result.err()
    );

    let (encoded_str, total_bytes, bytes_per_row) = result.unwrap();

    // Sanity checks on the output
    assert!(
        !encoded_str.is_empty(),
        "The resulting ZPL string should not be empty"
    );
    assert!(total_bytes > 0, "Total bytes should be greater than zero");
    assert!(
        bytes_per_row > 0,
        "Bytes per row should be greater than zero"
    );

    // Print metadata for debugging during test execution (run with --nocapture)
    println!(
        "^GFA,{},{},{},{}^FS",
        total_bytes.clone(),
        total_bytes.clone(),
        bytes_per_row,
        encoded_str
    );
    println!("Bytes per row: {}", bytes_per_row);
}
