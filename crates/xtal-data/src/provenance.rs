//! De dónde salió una medición: los bloques de trazabilidad de su `.toml`.
//!
//! ## Por qué esto es opaco a propósito
//!
//! Una medición se puede haber conseguido de muchas maneras: una fórmula, datos
//! sintéticos, una simulación de ngspice, un rawfile de LTspice, mañana quién sabe.
//! Xtal guarda **cómo se consiguió** al lado del dato, para poder rastrearla o
//! regenerarla.
//!
//! El problema es de quién es ese conocimiento. Mientras Xtal fue solo una herramienta
//! de electrónica, la capa de persistencia importaba los tipos del simulador para
//! poder escribir el bloque `[sim]`. O sea: el núcleo —el que guarda archivos— sabía
//! qué es un análisis AC de ngspice. Eso hacía que el motor de informes no se pudiera
//! usar sin arrastrar el simulador atrás.
//!
//! Acá se corta. `Provenance` es un mapa de bloques con nombre. El núcleo lo escribe y
//! lo devuelve **sin mirar adentro**. Quien produce la medición pone su bloque bajo su
//! propia clave: la fórmula pone `formula`, el addon de electrónica pone `sim` o `raw`.
//! Un addon nuevo pone lo suyo sin tocar una línea de este crate.
//!
//! El formato en disco no cambió ni un carácter: los bloques siguen siendo tablas de
//! primer nivel del `.toml` de la medición, igual que antes.

use serde::Serialize;

use crate::error::{DataError, Result};

/// Los bloques de trazabilidad de una medición, sin interpretar.
///
/// Se construye encadenando: `Provenance::new().with("formula", &spec)?`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Provenance(toml::Table);

impl Provenance {
    /// Sin trazabilidad. Es lo que corresponde a una medición importada de un CSV:
    /// el archivo original es la fuente, y no hay receta que guardar.
    pub fn new() -> Self {
        Self(toml::Table::new())
    }

    /// Agrega un bloque bajo `key`, serializando lo que le pases.
    ///
    /// Falla si el valor no serializa a una **tabla** TOML. Es a propósito: un bloque
    /// de trazabilidad tiene que ser un `[bloque]` con campos adentro, no un número
    /// suelto; si no, el `.toml` de la medición queda ilegible.
    pub fn with<T: Serialize>(mut self, key: &str, value: &T) -> Result<Self> {
        let valor =
            toml::Value::try_from(value).map_err(|e| DataError::Provenance(e.to_string()))?;
        if !valor.is_table() {
            return Err(DataError::Provenance(format!(
                "el bloque `{key}` tiene que ser una tabla"
            )));
        }
        self.0.insert(key.to_string(), valor);
        Ok(self)
    }

    /// Lo mismo, pero para un `Option`: si es `None` no agrega nada. Evita el
    /// `if let Some(...)` en cada sitio de llamada.
    pub fn with_opt<T: Serialize>(self, key: &str, value: Option<&T>) -> Result<Self> {
        match value {
            Some(v) => self.with(key, v),
            None => Ok(self),
        }
    }

    /// Un bloque por nombre, tal cual está en el archivo. El núcleo no lo usa; está
    /// para quien sí sepa qué hay adentro.
    pub fn get(&self, key: &str) -> Option<&toml::Value> {
        self.0.get(key)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// La tabla cruda, para serializar.
    pub(crate) fn table(&self) -> &toml::Table {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Falsa {
        circuito: String,
        vector: String,
    }

    #[test]
    fn guarda_y_devuelve_un_bloque_sin_saber_que_es() {
        let spec = Falsa {
            circuito: "rc".into(),
            vector: "v(out)".into(),
        };
        let p = Provenance::new().with("sim", &spec).unwrap();
        assert!(!p.is_empty());

        // El núcleo no sabe qué es un `sim`, pero lo devuelve entero.
        let vuelta: Falsa = p.get("sim").unwrap().clone().try_into().unwrap();
        assert_eq!(vuelta, spec);
    }

    #[test]
    fn sin_bloques_queda_vacia() {
        let p = Provenance::new();
        assert!(p.is_empty());
        assert!(p.get("sim").is_none());
        // `with_opt` con None no inventa nada.
        let p = p.with_opt::<Falsa>("sim", None).unwrap();
        assert!(p.is_empty());
    }

    #[test]
    fn un_bloque_que_no_es_tabla_se_rechaza() {
        // Un número suelto como `sim = 3` dejaría el .toml de la medición ilegible.
        let e = Provenance::new().with("sim", &42);
        assert!(e.is_err());
    }
}
