use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

pub trait CsvRecord {
    fn header() -> &'static [&'static str];
    fn row(&self) -> Vec<String>;
}

pub struct CsvWriter<W: Write> {
    writer: W,
}

impl CsvWriter<BufWriter<File>> {
    pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }
}

impl<W: Write> CsvWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn write_header(&mut self, header: &[&str]) -> io::Result<()> {
        self.write_row(header)
    }

    pub fn write_row<I, S>(&mut self, row: I) -> io::Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut first = true;
        for field in row {
            if !first {
                self.writer.write_all(b",")?;
            }
            first = false;
            let escaped = escape_field(field.as_ref());
            self.writer.write_all(escaped.as_bytes())?;
        }
        self.writer.write_all(b"\n")?;
        Ok(())
    }
}

pub fn write_csv<T: CsvRecord>(path: impl AsRef<Path>, records: &[T]) -> io::Result<()> {
    let mut writer = CsvWriter::create(path)?;
    writer.write_header(T::header())?;
    for record in records {
        writer.write_row(record.row())?;
    }
    Ok(())
}

fn escape_field(value: &str) -> String {
    let needs_quotes = value.contains(',')
        || value.contains('"')
        || value.contains('\n')
        || value.contains('\r');
    if !needs_quotes {
        return value.to_string();
    }

    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        if ch == '"' {
            out.push('"');
            out.push('"');
        } else {
            out.push(ch);
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_field_simple() {
        assert_eq!(escape_field("abc"), "abc");
    }

    #[test]
    fn escape_field_quotes_commas() {
        assert_eq!(escape_field("a,b"), "\"a,b\"");
        assert_eq!(escape_field("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn write_row_basic() {
        let mut out = Vec::new();
        let mut writer = CsvWriter::new(&mut out);
        writer.write_row(["a", "b", "c"]).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "a,b,c\n");
    }
}
