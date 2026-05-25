use clap::Parser;

#[derive(Parser)]
#[command(name = "artemis-export", about = "Convert Artemis .asb scripts to Bevy VN .bscript.ron")]
struct Args {
    #[arg(long)]
    input: String,
    #[arg(long)]
    output: String,
    #[arg(long, default_value_t = false)]
    verbose: bool,
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("Input: {}", args.input);
    println!("Output: {}", args.output);
    Ok(())
}
