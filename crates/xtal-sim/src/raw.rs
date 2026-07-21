//! Lectura de **rawfiles** de SPICE (LTspice y ngspice).
//!
//! El rawfile es el resultado de una corrida: lo que la persona obtiene cuando aprieta
//! "Run" en LTspice (o `write archivo.raw` en ngspice). Tiene un **encabezado de texto**
//! (`Title`, `Plotname`, `Flags`, las variables...) seguido de los **datos**, que pueden
//! venir en ASCII (`Values:`) o en binario (`Binary:`). Este módulo lo parsea a columnas
//! de `f64` (o pares re/im si es complejo), para que después se vuelvan `Measurement`s
//! igual que un CSV de osciloscopio o una simulación propia.
//!
//! ## Variantes que cubrimos (verificadas contra archivos reales)
//!
//! - **Encoding del encabezado**: UTF-8 (ngspice) o UTF-16LE (LTspice en Windows suele
//!   escribir el header en UTF-16, con o sin BOM). Se autodetecta.
//! - **ASCII vs binario**: marcador `Values:` (ASCII) o `Binary:` (binario).
//! - **Real vs complejo**: `Flags: real` → cada valor es un número; `Flags: complex`
//!   (análisis AC) → cada valor es un par (real, imag).
//! - **Ancho de los binarios** (el clásico bardo LTspice vs ngspice): para datos REALES,
//!   ngspice guarda TODO en `f64` (8 bytes), mientras que LTspice guarda la variable
//!   independiente (tiempo/frecuencia) en `f64` y el resto en `f32` (4 bytes). En vez de
//!   adivinar el simulador, **deducimos el layout por la longitud del blob**: comparamos
//!   los bytes que hay contra los dos tamaños posibles y elegimos el que encaja. Para
//!   datos COMPLEJOS, ambos simuladores usan dos `f64` por valor (16 bytes).

use crate::error::{Result, SimError};

/// Una columna de datos ya parseada: real (una lista de valores) o compleja (pares re/im).
#[derive(Debug, Clone, PartialEq)]
pub enum RawColumn {
    Real(Vec<f64>),
    Complex(Vec<(f64, f64)>),
}

/// Una variable del rawfile: su nombre (ej. `V(out)`) y su tipo SPICE (ej. `voltage`).
#[derive(Debug, Clone, PartialEq)]
pub struct RawVar {
    /// Nombre tal cual lo escribió el simulador (ej. `v(out)`, `time`, `frequency`).
    pub name: String,
    /// Tipo SPICE de la variable (`voltage`, `current`, `time`, `frequency`, ...).
    pub kind: String,
}

/// Un rawfile completo, ya parseado a memoria.
///
/// `vars[i]` describe la columna `columns[i]`. Por convención de SPICE, la variable 0 es
/// el **eje independiente** (tiempo en transitorio, frecuencia en AC, la fuente barrida
/// en un DC sweep).
#[derive(Debug, Clone)]
pub struct RawFile {
    /// Título del análisis (primera línea del header).
    pub title: String,
    /// Nombre del plot (ej. `Transient Analysis`, `AC Analysis`, `DC transfer ...`).
    pub plotname: String,
    /// ¿Los datos son complejos (análisis AC)?
    pub complex: bool,
    /// Cantidad de puntos por variable.
    pub n_points: usize,
    /// Las variables, en orden (la 0 es el eje independiente).
    pub vars: Vec<RawVar>,
    /// Una columna de datos por variable (misma longitud que `vars`).
    pub columns: Vec<RawColumn>,
}

/// Encoding del encabezado de texto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Enc {
    Utf8,
    Utf16Le,
}

impl RawFile {
    /// Parsea un rawfile desde sus bytes crudos.
    ///
    /// `force_double`: si es `true`, fuerza a interpretar los binarios reales como `f64`
    /// (todo doble). Sirve de escape si la autodetección por longitud fallara con algún
    /// archivo raro; en la práctica casi nunca hace falta.
    pub fn parse(bytes: &[u8], force_double: bool) -> Result<RawFile> {
        let enc = detect_enc(bytes);

        // Ubicamos el marcador que separa header de datos (`Binary:` o `Values:`).
        let (marker, data_start) = find_marker(bytes, enc)
            .ok_or_else(|| SimError::RawParse("no encontré 'Binary:' ni 'Values:' (¿es un rawfile de SPICE?)".into()))?;
        let header_text = decode_header(&bytes[..data_start], enc);
        let header = Header::parse(&header_text)?;

        let columns = match marker {
            Marker::Values => parse_ascii_values(
                &decode_header(&bytes[data_start..], enc),
                header.n_vars,
                header.n_points,
                header.complex,
            )?,
            Marker::Binary => parse_binary(
                &bytes[data_start..],
                header.n_vars,
                header.n_points,
                header.complex,
                force_double,
            )?,
        };

        Ok(RawFile {
            title: header.title,
            plotname: header.plotname,
            complex: header.complex,
            n_points: header.n_points,
            vars: header.vars,
            columns,
        })
    }

    /// Índice del eje independiente (siempre la variable 0 en SPICE).
    pub fn independent_index(&self) -> usize {
        0
    }

    /// El eje independiente como lista de `f64`. Si el archivo es complejo (AC), la
    /// frecuencia viene como par (f, 0); tomamos la parte real.
    pub fn x_values(&self) -> Vec<f64> {
        match &self.columns[0] {
            RawColumn::Real(v) => v.clone(),
            RawColumn::Complex(v) => v.iter().map(|(re, _)| *re).collect(),
        }
    }

    /// Busca una variable por nombre, tolerante: ignora mayúsculas y espacios, y matchea
    /// tanto el nombre completo (`v(out)`) como el nodo interno (`out`).
    pub fn find_var(&self, query: &str) -> Option<usize> {
        let q = normalize_name(query);
        self.vars.iter().position(|v| {
            let name = normalize_name(&v.name);
            name == q || inner_node(&name) == q
        })
    }
}

/// Mapea un tipo SPICE de variable a una unidad legible (para el eje del gráfico).
pub fn kind_to_unit(kind: &str) -> Option<&'static str> {
    match kind.to_ascii_lowercase().as_str() {
        "time" => Some("s"),
        "frequency" => Some("Hz"),
        "voltage" => Some("V"),
        "current" | "device_current" => Some("A"),
        _ => None,
    }
}

// --- Encabezado ---

/// Encabezado parseado del rawfile (sin los datos).
struct Header {
    title: String,
    plotname: String,
    complex: bool,
    n_vars: usize,
    n_points: usize,
    vars: Vec<RawVar>,
}

impl Header {
    fn parse(text: &str) -> Result<Header> {
        let mut title = String::new();
        let mut plotname = String::new();
        let mut complex = false;
        let mut n_vars = 0usize;
        let mut n_points = 0usize;

        let lines: Vec<&str> = text.lines().collect();
        let mut var_section: Option<usize> = None;

        for (i, line) in lines.iter().enumerate() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let value = value.trim();
            match key.trim().to_ascii_lowercase().as_str() {
                "title" => title = value.to_string(),
                "plotname" => plotname = value.to_string(),
                "flags" => complex = value.to_ascii_lowercase().contains("complex"),
                "no. variables" => {
                    n_vars = value
                        .parse()
                        .map_err(|_| SimError::RawParse(format!("'No. Variables' inválido: {value}")))?
                }
                "no. points" => {
                    n_points = value
                        .parse()
                        .map_err(|_| SimError::RawParse(format!("'No. Points' inválido: {value}")))?
                }
                "variables" => var_section = Some(i),
                _ => {}
            }
        }

        if n_vars == 0 {
            return Err(SimError::RawParse("header sin 'No. Variables'".into()));
        }
        if n_points == 0 {
            return Err(SimError::RawParse("header sin 'No. Points'".into()));
        }

        // Las definiciones de variables son las n_vars líneas que siguen a "Variables:".
        let start = var_section
            .ok_or_else(|| SimError::RawParse("header sin sección 'Variables:'".into()))?;
        let mut vars = Vec::with_capacity(n_vars);
        for line in lines.iter().skip(start + 1) {
            if vars.len() == n_vars {
                break;
            }
            let toks: Vec<&str> = line.split_whitespace().collect();
            // Cada def es: <idx> <nombre> <tipo> [extra...]. Saltamos líneas vacías.
            if toks.len() < 3 {
                continue;
            }
            vars.push(RawVar {
                name: toks[1].to_string(),
                kind: toks[2].to_string(),
            });
        }
        if vars.len() != n_vars {
            return Err(SimError::RawParse(format!(
                "esperaba {n_vars} variables, encontré {}",
                vars.len()
            )));
        }

        Ok(Header {
            title,
            plotname,
            complex,
            n_vars,
            n_points,
            vars,
        })
    }
}

// --- Detección de encoding y marcador ---

/// Heurística simple: BOM UTF-16LE, o muchos bytes 0x00 en el arranque → UTF-16LE.
fn detect_enc(bytes: &[u8]) -> Enc {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return Enc::Utf16Le;
    }
    let n = bytes.len().min(64);
    if n >= 4 {
        let zeros = bytes[..n].iter().filter(|b| **b == 0).count();
        // El header ASCII no tiene bytes nulos; el UTF-16LE tiene ~50%.
        if zeros * 4 >= n {
            return Enc::Utf16Le;
        }
    }
    Enc::Utf8
}

/// Qué marcador separa el header de los datos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Marker {
    Binary,
    Values,
}

/// Encuentra el marcador (`Binary:`/`Values:`) y devuelve dónde arrancan los datos
/// (después del salto de línea que sigue al marcador).
fn find_marker(bytes: &[u8], enc: Enc) -> Option<(Marker, usize)> {
    let bin = encode_needle("Binary:", enc);
    let val = encode_needle("Values:", enc);
    let pos_bin = find_sub(bytes, &bin).map(|p| (Marker::Binary, p, bin.len()));
    let pos_val = find_sub(bytes, &val).map(|p| (Marker::Values, p, val.len()));
    // El que aparezca primero en el archivo.
    let (marker, pos, len) = match (pos_bin, pos_val) {
        (Some(b), Some(v)) => {
            if b.1 <= v.1 {
                b
            } else {
                v
            }
        }
        (Some(b), None) => b,
        (None, Some(v)) => v,
        (None, None) => return None,
    };
    // Los datos empiezan después del primer '\n' (0x0A) que sigue al marcador.
    let after = pos + len;
    let lf = bytes[after..].iter().position(|b| *b == 0x0A)? + after;
    let mut data_start = lf + 1;
    // En UTF-16LE el '\n' es 0x0A 0x00: saltamos el byte nulo de cola.
    if enc == Enc::Utf16Le && bytes.get(data_start) == Some(&0x00) {
        data_start += 1;
    }
    Some((marker, data_start))
}

/// Codifica un texto buscador en el encoding dado (para localizar el marcador en bytes).
fn encode_needle(s: &str, enc: Enc) -> Vec<u8> {
    match enc {
        Enc::Utf8 => s.bytes().collect(),
        Enc::Utf16Le => s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect(),
    }
}

/// Búsqueda naive de subcadena en bytes (los headers son chicos, no hace falta más).
fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| &hay[i..i + needle.len()] == needle)
}

/// Decodifica una porción de bytes (header o datos ASCII) según el encoding.
fn decode_header(bytes: &[u8], enc: Enc) -> String {
    match enc {
        Enc::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        Enc::Utf16Le => {
            // Saltamos el BOM si está, juntamos pares LE en u16 y decodificamos.
            let start = if bytes.starts_with(&[0xFF, 0xFE]) { 2 } else { 0 };
            let units: Vec<u16> = bytes[start..]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&units)
        }
    }
}

// --- Parseo de datos ASCII ---

/// Parsea la sección `Values:` (ASCII). Formato de ngspice:
/// ```text
///  0\t<val0>      <- arranca el punto 0 (índice + primer valor)
/// \t<val1>        <- resto de variables, una por línea
/// \t<val2>
///                 <- línea en blanco entre puntos
///  1\t<val0>
/// ...
/// ```
/// Para datos complejos cada `<val>` es `re,im`.
fn parse_ascii_values(
    text: &str,
    n_vars: usize,
    n_points: usize,
    complex: bool,
) -> Result<Vec<RawColumn>> {
    // Aplanamos todos los valores en orden (n_points * n_vars tokens).
    let mut flat: Vec<&str> = Vec::with_capacity(n_points * n_vars);
    for line in text.lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        match toks.len() {
            // Línea de arranque de punto: [índice, valor0].
            2 => flat.push(toks[1]),
            // Línea de una variable suelta: [valor].
            1 => flat.push(toks[0]),
            _ => {} // líneas vacías o ruido
        }
    }

    let expected = n_points * n_vars;
    if flat.len() != expected {
        return Err(SimError::RawParse(format!(
            "datos ASCII: esperaba {expected} valores ({n_points} pts × {n_vars} vars), encontré {}",
            flat.len()
        )));
    }

    let mut columns = init_columns(n_vars, complex);
    for (k, tok) in flat.iter().enumerate() {
        let v = k % n_vars;
        push_value(&mut columns[v], tok, complex)?;
    }
    Ok(columns)
}

/// Inicializa `n_vars` columnas vacías del tipo correcto.
fn init_columns(n_vars: usize, complex: bool) -> Vec<RawColumn> {
    (0..n_vars)
        .map(|_| {
            if complex {
                RawColumn::Complex(Vec::new())
            } else {
                RawColumn::Real(Vec::new())
            }
        })
        .collect()
}

/// Empuja un valor ASCII a la columna (real → un f64; complejo → `re,im`).
fn push_value(col: &mut RawColumn, tok: &str, complex: bool) -> Result<()> {
    if complex {
        let (re, im) = tok
            .split_once(',')
            .ok_or_else(|| SimError::RawParse(format!("valor complejo sin coma: '{tok}'")))?;
        let re = parse_f64(re)?;
        let im = parse_f64(im)?;
        if let RawColumn::Complex(v) = col {
            v.push((re, im));
        }
    } else {
        let x = parse_f64(tok)?;
        if let RawColumn::Real(v) = col {
            v.push(x);
        }
    }
    Ok(())
}

fn parse_f64(s: &str) -> Result<f64> {
    s.trim()
        .parse::<f64>()
        .map_err(|_| SimError::RawParse(format!("número inválido: '{s}'")))
}

// --- Parseo de datos binarios ---

/// Parsea la sección `Binary:`. Ver la nota del módulo sobre los anchos.
fn parse_binary(
    blob: &[u8],
    n_vars: usize,
    n_points: usize,
    complex: bool,
    force_double: bool,
) -> Result<Vec<RawColumn>> {
    if complex {
        // Complejo: 2 × f64 por valor (16 bytes), todas las variables igual.
        let need = n_points * n_vars * 16;
        if blob.len() < need {
            return Err(SimError::RawParse(format!(
                "binario complejo: esperaba {need} bytes, hay {}",
                blob.len()
            )));
        }
        let mut columns = init_columns(n_vars, true);
        let mut off = 0;
        for _ in 0..n_points {
            for col in columns.iter_mut() {
                let re = read_f64(blob, off)?;
                let im = read_f64(blob, off + 8)?;
                off += 16;
                if let RawColumn::Complex(v) = col {
                    v.push((re, im));
                }
            }
        }
        return Ok(columns);
    }

    // Real: decidimos el layout por la longitud del blob. Preferimos el match EXACTO;
    // si sobran bytes (padding/cola), caemos al que entre. `mixed` = var0 f64 + resto f32
    // (convención LTspice); `all_f64` = todo doble (convención ngspice).
    let all_f64 = n_points * n_vars * 8;
    let mixed = n_points * (8 + (n_vars - 1) * 4);
    let var0_double_rest_single = if force_double {
        false
    } else if blob.len() == all_f64 {
        // Cubre también n_vars == 1 (donde all_f64 == mixed): se lee todo f64.
        false
    } else if blob.len() == mixed {
        true
    } else if blob.len() >= all_f64 {
        false
    } else if blob.len() >= mixed {
        true
    } else {
        return Err(SimError::RawParse(format!(
            "binario real: {} bytes no coinciden con f64 ({all_f64}) ni con mixto LTspice ({mixed})",
            blob.len()
        )));
    };

    let mut columns = init_columns(n_vars, false);
    let mut off = 0;
    for _ in 0..n_points {
        for (vi, col) in columns.iter_mut().enumerate() {
            // En el layout mixto, solo la var 0 es f64; el resto f32.
            let (value, size) = if var0_double_rest_single && vi != 0 {
                (read_f32(blob, off)? as f64, 4)
            } else {
                (read_f64(blob, off)?, 8)
            };
            off += size;
            if let RawColumn::Real(v) = col {
                v.push(value);
            }
        }
    }
    Ok(columns)
}

/// Lee un `f64` little-endian en `off` (con chequeo de límites).
fn read_f64(blob: &[u8], off: usize) -> Result<f64> {
    let end = off + 8;
    if end > blob.len() {
        return Err(SimError::RawParse("blob binario truncado (f64)".into()));
    }
    let mut b = [0u8; 8];
    b.copy_from_slice(&blob[off..end]);
    Ok(f64::from_le_bytes(b))
}

/// Lee un `f32` little-endian en `off` (con chequeo de límites).
fn read_f32(blob: &[u8], off: usize) -> Result<f32> {
    let end = off + 4;
    if end > blob.len() {
        return Err(SimError::RawParse("blob binario truncado (f32)".into()));
    }
    let mut b = [0u8; 4];
    b.copy_from_slice(&blob[off..end]);
    Ok(f32::from_le_bytes(b))
}

// --- Helpers de nombres ---

/// Normaliza un nombre de variable: minúsculas, sin espacios.
fn normalize_name(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Extrae el nodo interno de un nombre tipo `v(out)` → `out`. Si no tiene paréntesis,
/// devuelve el nombre tal cual.
fn inner_node(name: &str) -> String {
    match (name.find('('), name.rfind(')')) {
        (Some(a), Some(b)) if b > a + 1 => name[a + 1..b].to_string(),
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Header ASCII de juguete reusable.
    fn ascii_real() -> &'static str {
        "Title: rc\nPlotname: Transient Analysis\nFlags: real\nNo. Variables: 2\nNo. Points: 3\nVariables:\n\t0\ttime\ttime\n\t1\tv(out)\tvoltage\nValues:\n 0\t0.0\n\t1.0\n\n 1\t1.0\n\t2.0\n\n 2\t2.0\n\t3.0\n"
    }

    #[test]
    fn parses_ascii_real() {
        let raw = RawFile::parse(ascii_real().as_bytes(), false).unwrap();
        assert!(!raw.complex);
        assert_eq!(raw.n_points, 3);
        assert_eq!(raw.vars.len(), 2);
        assert_eq!(raw.vars[1].name, "v(out)");
        assert_eq!(raw.x_values(), vec![0.0, 1.0, 2.0]);
        assert_eq!(raw.columns[1], RawColumn::Real(vec![1.0, 2.0, 3.0]));
    }

    #[test]
    fn parses_ascii_complex() {
        let text = "Title: rc\nPlotname: AC Analysis\nFlags: complex\nNo. Variables: 2\nNo. Points: 2\nVariables:\n\t0\tfrequency\tfrequency\n\t1\tv(out)\tvoltage\nValues:\n 0\t10.0,0.0\n\t1.0,0.0\n\n 1\t100.0,0.0\n\t0.5,-0.5\n";
        let raw = RawFile::parse(text.as_bytes(), false).unwrap();
        assert!(raw.complex);
        assert_eq!(raw.x_values(), vec![10.0, 100.0]);
        assert_eq!(
            raw.columns[1],
            RawColumn::Complex(vec![(1.0, 0.0), (0.5, -0.5)])
        );
    }

    #[test]
    fn find_var_is_tolerant() {
        let raw = RawFile::parse(ascii_real().as_bytes(), false).unwrap();
        assert_eq!(raw.find_var("V(OUT)"), Some(1));
        assert_eq!(raw.find_var("out"), Some(1)); // nodo interno
        assert_eq!(raw.find_var("time"), Some(0));
        assert_eq!(raw.find_var("nope"), None);
    }

    // Construye un header binario + un blob dado.
    fn bin_file(header: &str, blob: &[u8]) -> Vec<u8> {
        let mut v = header.as_bytes().to_vec();
        v.extend_from_slice(blob);
        v
    }

    #[test]
    fn parses_binary_real_all_f64() {
        // 2 vars (time, v(out)), 2 puntos, todo f64 (estilo ngspice).
        let header = "Title: t\nPlotname: Transient Analysis\nFlags: real\nNo. Variables: 2\nNo. Points: 2\nVariables:\n\t0\ttime\ttime\n\t1\tv(out)\tvoltage\nBinary:\n";
        let mut blob = Vec::new();
        for v in [0.0_f64, 1.0, 1e-3, 2.0] {
            blob.extend_from_slice(&v.to_le_bytes());
        }
        let raw = RawFile::parse(&bin_file(header, &blob), false).unwrap();
        assert_eq!(raw.x_values(), vec![0.0, 1e-3]);
        assert_eq!(raw.columns[1], RawColumn::Real(vec![1.0, 2.0]));
    }

    #[test]
    fn parses_binary_real_ltspice_mixed() {
        // LTspice: var0 (time) f64, resto f32. 3 vars, 2 puntos.
        let header = "Title: t\nPlotname: Transient Analysis\nFlags: real\nNo. Variables: 3\nNo. Points: 2\nVariables:\n\t0\ttime\ttime\n\t1\tv(out)\tvoltage\n\t2\tv(in)\tvoltage\nBinary:\n";
        let mut blob = Vec::new();
        // punto 0: t=0 (f64), vout=1 (f32), vin=5 (f32)
        blob.extend_from_slice(&0.0_f64.to_le_bytes());
        blob.extend_from_slice(&1.0_f32.to_le_bytes());
        blob.extend_from_slice(&5.0_f32.to_le_bytes());
        // punto 1: t=1e-3, vout=2, vin=5
        blob.extend_from_slice(&1e-3_f64.to_le_bytes());
        blob.extend_from_slice(&2.0_f32.to_le_bytes());
        blob.extend_from_slice(&5.0_f32.to_le_bytes());
        let raw = RawFile::parse(&bin_file(header, &blob), false).unwrap();
        assert_eq!(raw.x_values(), vec![0.0, 1e-3]);
        assert_eq!(raw.columns[1], RawColumn::Real(vec![1.0, 2.0]));
        assert_eq!(raw.columns[2], RawColumn::Real(vec![5.0, 5.0]));
    }

    #[test]
    fn parses_binary_complex() {
        // AC: 2 vars, 2 puntos, 16 bytes por valor.
        let header = "Title: t\nPlotname: AC Analysis\nFlags: complex\nNo. Variables: 2\nNo. Points: 2\nVariables:\n\t0\tfrequency\tfrequency\n\t1\tv(out)\tvoltage\nBinary:\n";
        let mut blob = Vec::new();
        for (re, im) in [(10.0_f64, 0.0_f64), (1.0, 0.0), (100.0, 0.0), (0.5, -0.5)] {
            blob.extend_from_slice(&re.to_le_bytes());
            blob.extend_from_slice(&im.to_le_bytes());
        }
        let raw = RawFile::parse(&bin_file(header, &blob), false).unwrap();
        assert_eq!(raw.x_values(), vec![10.0, 100.0]);
        assert_eq!(
            raw.columns[1],
            RawColumn::Complex(vec![(1.0, 0.0), (0.5, -0.5)])
        );
    }

    #[test]
    fn detects_utf16le_header() {
        // Mismo contenido real, pero header en UTF-16LE con BOM y datos ASCII.
        let text = ascii_real();
        let mut bytes = vec![0xFF, 0xFE];
        for u in text.encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        let raw = RawFile::parse(&bytes, false).unwrap();
        assert_eq!(raw.x_values(), vec![0.0, 1.0, 2.0]);
        assert_eq!(raw.columns[1], RawColumn::Real(vec![1.0, 2.0, 3.0]));
    }

    #[test]
    fn kind_units() {
        assert_eq!(kind_to_unit("voltage"), Some("V"));
        assert_eq!(kind_to_unit("time"), Some("s"));
        assert_eq!(kind_to_unit("frequency"), Some("Hz"));
        assert_eq!(kind_to_unit("current"), Some("A"));
        assert_eq!(kind_to_unit("bogus"), None);
    }
}
