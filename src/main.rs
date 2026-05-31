use clap::Parser;
use lepton_jpeg::{DEFAULT_THREAD_POOL, EnabledFeatures};
use preflate_container::{
    PreflateContainerConfig, PreflateContainerProcessor, ProcessBuffer, RecreateContainerProcessor,
};
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rawflate")]
#[command(about = "Preflate PNG/JPEG to raw data for zpaq, and restore")]
struct Cli {
    #[arg(short, long)]
    mode: String,

    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long)]
    output: PathBuf,
}

fn encode_jpeg(data: &[u8], output: &mut impl Write) -> Result<(), String> {
    let mut encoded = Vec::new();
    lepton_jpeg::encode_lepton(
        &mut Cursor::new(data),
        &mut Cursor::new(&mut encoded),
        &EnabledFeatures::compat_lepton_vector_write(),
        &DEFAULT_THREAD_POOL,
    )
    .map_err(|e| format!("Lepton encode: {}", e))?;
    output.write_all(&[0x01]).unwrap();
    output
        .write_all(&(data.len() as u32).to_le_bytes())
        .unwrap();
    output.write_all(&encoded).unwrap();
    Ok(())
}

fn decode_jpeg(
    input: &mut Cursor<&[u8]>,
    size: usize,
    output: &mut impl Write,
) -> Result<(), String> {
    let mut lepton = vec![0u8; size];
    input.read_exact(&mut lepton).unwrap();
    lepton_jpeg::decode_lepton(
        &mut Cursor::new(&lepton),
        output,
        &EnabledFeatures::compat_lepton_vector_read(),
        &DEFAULT_THREAD_POOL,
    )
    .map_err(|e| format!("Lepton decode: {}", e))?;
    Ok(())
}

fn encode_png(data: &[u8], output: &mut impl Write) -> Result<(), String> {
    let config = PreflateContainerConfig::default();
    let mut processor = PreflateContainerProcessor::new(&config, 1, false);
    output.write_all(&[0x02]).unwrap();
    output
        .write_all(&(data.len() as u32).to_le_bytes())
        .unwrap();
    processor
        .process_buffer(data, true, output)
        .map_err(|e| format!("Preflate encode: {}", e))?;
    Ok(())
}

fn decode_png(
    input: &mut Cursor<&[u8]>,
    size: usize,
    output: &mut impl Write,
) -> Result<(), String> {
    let mut container = vec![0u8; size];
    input.read_exact(&mut container).unwrap();
    let mut processor = RecreateContainerProcessor::new(128 * 1024 * 1024);
    processor
        .process_buffer(&container, true, output)
        .map_err(|e| format!("Preflate decode: {}", e))?;
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let mut input_data = Vec::new();
    std::fs::File::open(&cli.input)
        .unwrap_or_else(|e| {
            eprintln!("无法打开输入: {} ({})", cli.input.display(), e);
            std::process::exit(1);
        })
        .read_to_end(&mut input_data)
        .unwrap();
    let mut output = std::fs::File::create(&cli.output).unwrap_or_else(|e| {
        eprintln!("无法创建输出: {} ({})", cli.output.display(), e);
        std::process::exit(1);
    });

    let result = match cli.mode.as_str() {
        "encode" => {
            let mut cursor = Cursor::new(input_data.as_slice());
            match detect_format(&mut cursor) {
                "jpeg" => encode_jpeg(&input_data, &mut output),
                "png" => encode_png(&input_data, &mut output),
                other => Err(format!("不支持格式: {}", other)),
            }
        }
        "decode" => {
            let mut cursor = Cursor::new(input_data.as_slice());
            let mut buf = [0u8; 1];
            cursor.read_exact(&mut buf).unwrap();
            // read 4-byte LE total_size (total = remaining after header)
            let mut sz = [0u8; 4];
            cursor.read_exact(&mut sz).unwrap();
            let _original = u32::from_le_bytes(sz) as usize;
            let remaining = input_data.len() - 5;
            match buf[0] {
                0x01 => decode_jpeg(&mut cursor, remaining, &mut output),
                0x02 => decode_png(&mut cursor, remaining, &mut output),
                b => Err(format!("未知类型: 0x{:02x}", b)),
            }
        }
        _ => {
            eprintln!("模式应为 'encode' 或 'decode'");
            std::process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }
}

const JPEG_MAGIC: [u8; 2] = [0xFF, 0xD8];
const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

fn detect_format(cursor: &mut Cursor<&[u8]>) -> &'static str {
    let mut buf = [0u8; 8];
    match cursor.read_exact(&mut buf) {
        Ok(()) => {}
        Err(_) => return "unknown",
    }
    if buf[..2] == JPEG_MAGIC {
        "jpeg"
    } else if buf[..8] == PNG_MAGIC {
        "png"
    } else {
        "unknown"
    }
}
