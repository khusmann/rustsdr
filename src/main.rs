use core::panic;

use clap::{Parser, Subcommand};

use rustsdr::{noise, tone};

use tokio_stream::StreamExt;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
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
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    let stream = match &cli.command {
        Some(Commands::Tone {
            freq,
            rate,
            amplitude,
        }) => tone(freq, rate, amplitude),
        Some(Commands::Noise {}) => noise(),
        None => panic!("No subcommand provided"),
    };

    let mut stream = stream.take(5);

    while let Some(v) = stream.next().await {
        println!("GOT = {:?}", v);
    }
}
