use rustsdr::bss::IntoDynSampleStream;
use rustsdr::*;

use core::panic;

use clap::{Parser, Subcommand, ValueEnum};

use bss::{
    ComplexBufferedSampleStream, ConvertComplexStream, ConvertRealStream, RealBufferedSampleStream,
};

use tokio_stream::StreamExt;

use tokio::io::{stdout, AsyncWriteExt};
use tokio::pin;

/*
cargo run -- tone -a 0.5 -f 440 -r 48000 |
cargo run -- convert --input char --output s16 |
mplayer -cache 1024 -quiet -rawaudio samplesize=2:channels=1:rate=48000 -demuxer rawaudio -
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
        input: BitDepthOpt,
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
    let pipe = bss::from_sample_fn(gen::tone_fn(440, 48000, 1.0), 10, 10)
        .realpart()
        .lift_complex()
        .convert_to_char()
        .realpart()
        .convert_to_s16()
        .into_dyn()
        .convert(BitDepthOpt::Char.into());

    let mut stream = pipe.serialize();

    while let Some(v) = stream.next().await {
        println!("{:?}", v)
    }
    /*
        let cli = Cli::parse();

        let stream = match &cli.command {
            Some(Commands::Tone {
                freq,
                rate,
                amplitude,
            }) => source_tone(*freq, *rate, *amplitude, cli.buffer_size),
            Some(Commands::Noise { rate }) => source_noise(*rate, cli.buffer_size),
            Some(_) => source_stdin(cli.buffer_size),
            None => panic!("No subcommand provided"),
        };

        let mut stream = match &cli.command {
            Some(Commands::Convert { input, output }) => {
                Box::pin(stream.map(|v| v.map(convert_fn(input.to_bitdepth(), output.to_bitdepth()))))
            }
            Some(_) => stream,
            None => panic!("No subcommand provided"),
        };

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
    */
    Ok(())
}
