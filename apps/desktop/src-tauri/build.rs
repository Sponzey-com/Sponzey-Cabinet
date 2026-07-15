fn main() {
    ensure_default_icon();
    tauri_build::build();
}

fn ensure_default_icon() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR must be set for desktop build");
    let icons_dir = std::path::Path::new(&manifest_dir).join("icons");
    let icon_path = icons_dir.join("icon.png");

    std::fs::create_dir_all(&icons_dir).expect("desktop icon directory must be created");
    std::fs::write(icon_path, create_default_icon_png())
        .expect("default desktop icon must be written");
}

fn create_default_icon_png() -> Vec<u8> {
    const SIZE: u32 = 32;
    let mut rgba_rows = Vec::new();
    for y in 0..SIZE {
        rgba_rows.push(0);
        for x in 0..SIZE {
            let inside_mark = (9..=22).contains(&x) && (9..=22).contains(&y);
            let pixel = if inside_mark {
                [255, 255, 255, 255]
            } else {
                [37, 99, 235, 255]
            };
            rgba_rows.extend_from_slice(&pixel);
        }
    }

    let mut png = Vec::new();
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&SIZE.to_be_bytes());
    ihdr.extend_from_slice(&SIZE.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    append_png_chunk(&mut png, b"IHDR", &ihdr);

    append_png_chunk(&mut png, b"IDAT", &zlib_store(&rgba_rows));
    append_png_chunk(&mut png, b"IEND", &[]);
    png
}

fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut output = vec![0x78, 0x01];
    let mut offset = 0;
    while offset < data.len() {
        let remaining = data.len() - offset;
        let block_len = remaining.min(u16::MAX as usize);
        let is_final = offset + block_len == data.len();
        output.push(if is_final { 0x01 } else { 0x00 });
        let len = block_len as u16;
        output.extend_from_slice(&len.to_le_bytes());
        output.extend_from_slice(&(!len).to_le_bytes());
        output.extend_from_slice(&data[offset..offset + block_len]);
        offset += block_len;
    }
    output.extend_from_slice(&adler32(data).to_be_bytes());
    output
}

fn append_png_chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    png.extend_from_slice(kind);
    png.extend_from_slice(data);

    let mut crc_input = Vec::with_capacity(kind.len() + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    png.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let mut a = 1;
    let mut b = 0;
    for byte in data {
        a = (a + u32::from(*byte)) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}
