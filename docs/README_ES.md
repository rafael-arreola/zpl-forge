# ZPL-Forge

[![Crates.io](https://img.shields.io/crates/v/zpl_forge.svg)](https://crates.io/crates/zpl_forge)
[![Docs.rs](https://docs.rs/zpl-forge/badge.svg)](https://docs.rs/zpl-forge)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/rafael-arreola/zpl-forge#license)

[English](../README.md) | [Español]

`zpl-forge` es un motor de alto rendimiento escrito en Rust para parsear, procesar y renderizar etiquetas en formato Zebra Programming Language (ZPL). El proyecto transforma cadenas ZPL crudas en una representación intermedia (IR) optimizada, permitiendo su exportación a diversos formatos como imágenes PNG o documentos PDF.

## Características Principales

- **Arquitectura Basada en AST**: Utiliza `nom` para un parseo robusto y eficiente de comandos ZPL.
- **Motor de Estado**: Convierte el flujo de comandos en una lista de instrucciones autocontenidas, manejando el estado global de la etiqueta (fuentes, posiciones, etc.).
- **Backends Flexibles**: Soporte nativo para renderizado en PNG (vía `imageproc`) y PDF (vía `printpdf`).
- **Extensibilidad**: Comandos personalizados para soporte de color y carga de imágenes externas.
- **Rendimiento**: Diseñado para minimizar asignaciones y ser seguro en entornos concurrentes.

## Comandos ZPL Soportados

A continuación se listan los comandos que están actualmente implementados y operativos:

| Comando | Nombre           | Parámetros    | Descripción                                                                             |
| :------ | :--------------- | :------------ | :-------------------------------------------------------------------------------------- |
| `^A`    | Font Spec        | `f,o,h,w`     | Especifica la fuente (A..Z, 0..9), orientación (N, R, I, B), altura y ancho en puntos.  |
| `^B3`   | Code 39          | `o,e,h,f,g`   | Código de barras Code 39.                                                               |
| `^BC`   | Code 128         | `o,h,f,g,e,m` | Código de barras Code 128.                                                              |
| `^BQ`   | QR Code          | `o,m,s,e,k`   | Código QR (Modelo 1 o 2).                                                               |
| `^BY`   | Barcode Default  | `w,r,h`       | Establece valores por defecto para códigos de barras (ancho de módulo, ratio y altura). |
| `^CF`   | Change Def. Font | `f,h,w`       | Cambia la fuente alfanumérica por defecto.                                              |
| `^FD`   | Field Data       | `d`           | Datos a imprimir en el campo actual.                                                    |
| `^FO`   | Field Origin     | `x,y`         | Establece la coordenada superior izquierda del campo.                                   |
| `^FR`   | Field Reverse    | N/A           | Invierte el color del campo (blanco sobre negro).                                       |
| `^FS`   | Field Separator  | N/A           | Indica el final de una definición de campo.                                             |
| `^FT`   | Field Typeset    | `x,y`         | Establece la posición del campo relativa a la línea base del texto.                     |
| `^GB`   | Graphic Box      | `w,h,t,c,r`   | Dibuja una caja, línea o rectángulo con bordes redondeados.                             |
| `^GC`   | Graphic Circle   | `d,t,c`       | Dibuja un círculo especificando su diámetro.                                            |
| `^GE`   | Graphic Ellipse  | `w,h,t,c`     | Dibuja una elipse.                                                                      |
| `^GF`   | Graphic Field    | `c,b,f,p,d`   | Renderiza una imagen bitmap (soporta compresión tipo A/Hex).                            |
| `^XA`   | Start Format     | N/A           | Indica el inicio de una etiqueta.                                                       |
| `^XZ`   | End Format       | N/A           | Indica el final de la etiqueta.                                                         |

## Comandos Custom (Extensiones)

| Comando | Nombre       | Parámetros | Descripción                                                                                                 |
| :------ | :----------- | :--------- | :---------------------------------------------------------------------------------------------------------- |
| `^GIC`  | Custom Image | `w,h,d`    | Renderiza una imagen a color. **w** y **h** definen el tamaño. **d** es el binario (PNG/JPG) en **Base64**. |
| `^GLC`  | Line Color   | `c`        | Establece el color para elementos gráficos en formato hexadecimal (ej. `#FF0000`).                          |
| `^GTC`  | Text Color   | `c`        | Establece el color para los campos de texto en formato hexadecimal (ej. `#0000FF`).                         |

## Instalación

Agrega esto a tu `Cargo.toml`:

```toml
[dependencies]
zpl-forge = "0.1.0"
```

## Uso

### Renderizado a PNG

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::png::PngBackend;

fn main() {
    let zpl_input = "^XA^FO50,50^A0N,50,50^FDZPL Forge^FS^XZ";

    let engine = ZplEngine::new(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(2.0),
        Resolution::Dpi203
    ).expect("Error al parsear ZPL");

    let png_backend = PngBackend::new();
    let png_bytes = engine.render(png_backend, &HashMap::new())
        .expect("Error al renderizar");

    std::fs::write("output.png", png_bytes).ok();
}
```

### Renderizado a PDF

```rust
use std::collections::HashMap;
use zpl_forge::{ZplEngine, Unit, Resolution};
use zpl_forge::forge::pdf::PdfBackend;

fn main() {
    let zpl_input = "^XA^FO50,50^A0N,50,50^FDZPL Forge^FS^XZ";

    let engine = ZplEngine::new(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(2.0),
        Resolution::Dpi203
    ).expect("Error al parsear ZPL");

    let pdf_backend = PdfBackend::new();
    let pdf_bytes = engine.render(pdf_backend, &HashMap::new())
        .expect("Error al renderizar");

    std::fs::write("output.pdf", pdf_bytes).ok();
}
```

### Uso de Fuentes Personalizadas

Puedes cargar y usar tus propias fuentes TrueType (`.ttf`) o OpenType (`.otf`) registrándolas con el `FontManager` antes de renderizar.

```rust
use std::sync::Arc;
use zpl_forge::{ZplEngine, FontManager, Unit, Resolution};

fn main() -> zpl_forge::ZplResult<()> {
    let mut font_manager = FontManager::default();

    // 1. Cargar los bytes de la fuente desde un archivo o incluirlos en tiempo de compilación
    let font_bytes = std::fs::read("fuentes/Roboto-Regular.ttf")
        .expect("Archivo de fuente no encontrado");

    // 2. Registrar la fuente y mapearla a un rango de identificadores ZPL (ej. A-Z y 0-9)
    font_manager.register_font("Roboto", &font_bytes, 'A', '9')?;

    let zpl_input = "^XA^FO50,50^AAN,50,50^FDTexto con fuente Roboto^FS^XZ";
    let mut engine = ZplEngine::new(
        zpl_input,
        Unit::Inches(4.0),
        Unit::Inches(2.0),
        Resolution::Dpi203
    )?;

    // 3. Proporcionar el gestor de fuentes personalizado al motor
    engine.set_fonts(Arc::new(font_manager));

    // 4. Renderizar a PNG o PDF
    // ...
    Ok(())
}
```

## Seguridad y Límites

Para garantizar la estabilidad y prevenir ataques de denegación de servicio (DoS) por agotamiento de memoria, `zpl-forge` implementa las siguientes restricciones:

- **Tamaño del Lienzo**: El renderizado está limitado a un máximo de **8192 x 8192 píxeles**.
- **Imágenes ZPL (`^GF`)**: Los datos decodificados de imágenes no pueden exceder los **10 MB** por comando.
- **Aritmética Segura**: Se utiliza aritmética saturada para todos los cálculos de coordenadas y dimensiones, evitando desbordamientos de enteros.
- **Validación de Unidades**: Los valores de entrada para dimensiones físicas (pulgadas, mm, cm) se normalizan para evitar valores negativos.

## Licencia

Este proyecto está bajo la licencia MIT o Apache-2.0.
