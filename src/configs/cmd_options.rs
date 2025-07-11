use clap::Parser;

#[derive(Debug, Parser)]
pub struct CmdOptions {
    #[arg(short = 'd', long = "dir", default_value = "/tmp/redis-files")]
    pub dir: String,

    #[arg(short = 'f', long = "dbfilename", default_value = "dump.rdb")]
    pub filename: String,

    #[arg(short = 'u', long = "host", default_value = "0.0.0.0")]
    pub host: String,

    #[arg(short = 'p', long = "port", default_value = "6379")]
    pub port: String,

    #[arg(short, long = "replicaof", value_parser = valid_replicaof)]
    pub replicatof: Option<String>,
}

fn valid_replicaof(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Err("Replica master address cannot be empty".to_string());
    }

    let str = value.trim().replace(" ", ":");

    Ok(str.to_string())
}
