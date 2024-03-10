use core::panic;

use clap::{Parser, Subcommand};

use rustsdr::{source_noise, source_stdin, source_tone};

use tokio_stream::StreamExt;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = 1024)]
    buffer_size: usize,
    #[command(subcommand)]
    command: Option<Commands>,
}

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
    Convert,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut stream = match &cli.command {
        Some(Commands::Tone {
            freq,
            rate,
            amplitude,
        }) => source_tone(freq, rate, amplitude, cli.buffer_size),
        Some(Commands::Noise {}) => source_noise(cli.buffer_size),
        Some(_) => source_stdin(cli.buffer_size),
        None => panic!("No subcommand provided"),
    };

    //let mut stream = stream.take(100);

    while let Some(v) = stream.next().await {
        println!("GOT = {:?}", v);
    }
}
