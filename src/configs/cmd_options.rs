use clap::Parser;

#[derive(Debug, Parser)]
pub struct CmdOptions {
    #[arg(short = 'd', long = "dir", default_value = "/tmp/redis-files")]
    pub dir: String,

    #[arg(short = 'f', long = "dbfilename", default_value = "dump.rdb")]
    pub filename: String,

    #[arg(short = 'h', long = "host", default_value = "127.0.0.1")]
    pub host: String,

    #[arg(short = 'p', long = "port", default_value = "6379")]
    pub port: String,
}
