const ENCODING_ASCII: u8 = 0x00;
const ENCODING_UTF16_BOM: u8 = 0x01;
const ENCODING_UTF16_BE: u8 = 0x02;
const ENCODING_UTF8: u8 = 0x03;

#[derive(Clone, Copy, Debug)]
pub enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Utf16Bom,
}

impl Encoding {
    pub fn from_raw(flag: u8) -> Encoding {
        match flag {
            // ASCII and UTF8 can be mapped to the same type
            ENCODING_ASCII | ENCODING_UTF8 => Encoding::Utf8,

            // UTF16 with BOM [Can be both LE or BE]
            ENCODING_UTF16_BOM => Encoding::Utf16Bom,

            // UTF16 without BOM [Always BE]
            ENCODING_UTF16_BE => Encoding::Utf16Be,

            // Malformed, just say its UTF-8 and hope for the best
            _ => Encoding::Utf8,
        }
    }
}

pub fn get_string(encoding: Encoding, data: &[u8]) -> String {
    return match encoding {
        Encoding::Utf8 => String::from_utf8_lossy(data).to_string(),

        // LE isn't part of the spec, but it's needed when a BOM needs to be
        // re-used
        Encoding::Utf16Le => str_from_utf16le(data),

        Encoding::Utf16Be => str_from_utf16be(data),

        // UTF16BOM requires us to figure out the endianness ourselves from the BOM
        Encoding::Utf16Bom => match (data[0], data[1]) {
            (0xFF, 0xFE) => str_from_utf16le(&data[2..]), // Little Endian
            (0xFE, 0xFF) => str_from_utf16be(&data[2..]), // Big Endian
            _ => str_from_utf16ne(data),                  // No BOM, use native UTF-16
        },
    };
}

pub fn get_terminated_string(encoding: Encoding, data: &[u8]) -> (String, usize) {
    // Search for the NUL terminator, which is 0x00 in UTF-8 and 0x0000 in UTF-16
    // The string data will not include the terminator, but the size will.
    let (string_data, size) = match encoding {
        Encoding::Utf8 => slice_nul_utf8(data),
        _ => slice_nul_utf16(data),
    };

    let string = get_string(encoding, string_data);

    (string, size)
}

fn slice_nul_utf8(data: &[u8]) -> (&[u8], usize) {
    let mut size = 0;

    loop {
        if size >= data.len() {
            return (&data[0..size], size);
        }

        if data[size] == 0 {
            return (&data[0..size], size + 1);
        }

        size += 1;
    }
}

fn slice_nul_utf16(data: &[u8]) -> (&[u8], usize) {
    let mut size = 0;

    loop {
        if size + 1 > data.len() {
            return (&data[0..size], size);
        }

        if data[size] == 0 && data[size + 1] == 0 {
            return (&data[0..size], size + 2);
        }

        size += 2;
    }
}

fn str_from_utf16le(data: &[u8]) -> String {
    let result: Vec<u16> = data
        .chunks_exact(2)
        .into_iter()
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();

    return String::from_utf16_lossy(&result.as_slice());
}

fn str_from_utf16be(data: &[u8]) -> String {
    let result: Vec<u16> = data
        .chunks_exact(2)
        .into_iter()
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect();

    return String::from_utf16_lossy(&result.as_slice());
}

fn str_from_utf16ne(data: &[u8]) -> String {
    let result: Vec<u16> = data
        .chunks_exact(2)
        .into_iter()
        .map(|pair| u16::from_ne_bytes([pair[0], pair[1]]))
        .collect();

    return String::from_utf16_lossy(&result.as_slice());
}