use clap::{Parser, Subcommand};

/// A simple CLI tool with subcommands
#[derive(Parser, Debug)]
#[command(name = "cli-tool")]
#[command(about = "An example CLI tool using clap with subcommands")]
pub struct Cli {
    #[arg(short, long, default_value = "127.0.0.1:8080")]
    pub connect_to: String,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Upload a file
    Upload {
        /// The name of your package
        #[arg(value_name = "NAME", required = true)]
        name: String,
        /// The file to upload
        #[arg(value_name = "FILE", required = true, num_args = 1)]
        file: Vec<String>,
    },
    /// Download a file
    Download {
        /// The file to download
        #[arg(value_name = "FILE")]
        file: Vec<String>,
    },
    /// List files
    List,
}
