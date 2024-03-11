use core::panic;

use clap::{Parser, Subcommand, ValueEnum};

use rustsdr::{source_noise, source_stdin, source_tone};

use tokio_stream::StreamExt;

use tokio::io::{stdout, AsyncWriteExt};

use tokio_util::bytes::{BufMut, BytesMut};

/*
cargo run -- -b 10000 tone -a 0.5 -f 440 -r 48000 | mplayer -cache 1024 -quiet -rawaudio samplesize=2:channels=1:rate=48000 -demuxer rawaudio -
*/

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = 1024)]
    buffer_size: usize,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(ValueEnum, Debug, Clone)]
enum BitDepthOpt {
    Char,
    S16,
    Float,
}

/*
impl BitDepthOpt {
    fn as_internal(&self) -> rustsdr::BitDepth {
        match self {
            BitDepthOpt::Char => rustsdr::BitDepth::Char,
            BitDepthOpt::S16 => rustsdr::BitDepth::S16,
            BitDepthOpt::Float => rustsdr::BitDepth::Float,
        }
    }
}
*/

#[derive(Subcommand)]
enum Commands {
    Noise {},
    Tone {
        #[arg(short, long)]
        freq: u32,
        #[arg(short, long)]
        rate: u32,
        #[arg(short, long, default_value_t = 1.0)]
        amplitude: f32,
    },
    Convert {
        #[arg(short, long, value_enum)]
        input: BitDepthOpt,
        #[arg(short, long, value_enum)]
        output: BitDepthOpt,
    },
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    let stream = match &cli.command {
        Some(Commands::Tone {
            freq,
            rate,
            amplitude,
        }) => source_tone(freq, rate, amplitude, cli.buffer_size),
        Some(Commands::Noise {}) => source_noise(cli.buffer_size),
        Some(_) => source_stdin(cli.buffer_size),
        None => panic!("No subcommand provided"),
    };

    let mut stream = stream.map(|v| {
        v.map(|v| {
            let mut buf = BytesMut::new();

            for b in v {
                buf.put_u16(b as u16 * (u16::MAX / u8::MAX as u16))
            }

            buf
        })
    });

    while let Some(v) = stream.next().await {
        match v {
            Ok(v) => {
                stdout().write_all(&v).await?;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(())
}
