use rustsdr::*;

use core::panic;

use clap::{Parser, Subcommand, ValueEnum};

// use bss::DynSampleStream;

use tokio_stream::StreamExt;

use tokio::io::{stdout, AsyncWriteExt};

/*
cargo run -- -i float -n real tone -r 48000 -f 440 -a 1 |
cargo run -- -i float -n real convert --output s16 |
mplayer -cache 1024 -quiet -rawaudio samplesize=2:channels=1:rate=48000 -demuxer rawaudio -
*/

/*
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = 1024)]
    buffer_size: usize,
    #[arg(short, long, value_enum)]
    input: BitDepthOpt,
    #[arg(short, long, value_enum)]
    num_type: NumTypeOpt,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum NumTypeOpt {
    Real,
    Complex,
}

impl From<NumTypeOpt> for bss::NumType {
    fn from(opt: NumTypeOpt) -> bss::NumType {
        match opt {
            NumTypeOpt::Real => bss::NumType::Real,
            NumTypeOpt::Complex => bss::NumType::Complex,
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum BitDepthOpt {
    Char,
    S16,
    Float,
}

impl From<BitDepthOpt> for bss::BitDepth {
    fn from(opt: BitDepthOpt) -> bss::BitDepth {
        match opt {
            BitDepthOpt::Char => bss::BitDepth::Char,
            BitDepthOpt::S16 => bss::BitDepth::S16,
            BitDepthOpt::Float => bss::BitDepth::Float,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    Noise {
        #[arg(short, long)]
        rate: u32,
    },
    Tone {
        #[arg(short, long)]
        freq: u32,
        #[arg(short, long)]
        rate: u32,
        #[arg(short, long, default_value_t = 1.0, value_parser = parse_amplitude)]
        amplitude: f32,
    },
    Convert {
        #[arg(short, long, value_enum)]
        output: BitDepthOpt,
    },
}

fn parse_amplitude(s: &str) -> Result<f32, String> {
    let port: f32 = s.parse().map_err(|_| format!("`{s}` isn't a number"))?;

    if port >= 0.0 && port <= 1.0 {
        Ok(port)
    } else {
        Err(format!("amplitude not in range 0.0 - 1.0"))
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    let pipeline = match &cli.command {
        Some(Commands::Tone {
            freq,
            rate,
            amplitude,
        }) => DynSampleStream::source_tone(
            *freq,
            *rate,
            *amplitude,
            cli.buffer_size,
            cli.num_type.into(),
            cli.input.into(),
        ),
        Some(Commands::Noise { rate }) => DynSampleStream::source_noise(
            *rate,
            cli.buffer_size,
            cli.num_type.into(),
            cli.input.into(),
        ),
        Some(Commands::Convert { output }) => {
            DynSampleStream::source_stdin(cli.buffer_size, cli.num_type.into(), cli.input.into())
                .convert(output.clone().into())
        }
        None => panic!("No subcommand provided"),
    };

    let mut stream = pipeline.stream_bytes();

    while let Some(v) = stream.next().await {
        stdout().write_all(v.as_ref()).await?;
    }
    Ok(())
}

*/
#[tokio::main]
async fn main() -> std::io::Result<()> {
    Ok(())
}
