use anyhow::Context;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::configs::configurations::Configuration;
use crate::persistence::rdb::RDB;
use crate::redis_service::RedisService;

pub async fn listen(config: Configuration) -> anyhow::Result<()> {
    let address = config.get_address();

    let rdb = RDB::new(&config.get_rdb_path())?;

    let service = Arc::new(RedisService::new(config, Box::new(rdb)).await);

    let listener = TcpListener::bind(address.as_str())
        .await
        .with_context(|| format!("Could not listen on {}", address))
        .unwrap();

    println!("Redis Server started listening on {}", address);

    loop {
        let stream = listener.accept().await;

        match stream {
            Ok((mut stream, _)) => {
                let service_clone = Arc::clone(&service);

                tokio::spawn(async move {
                    let result = handle_connection(&mut stream, service_clone).await;

                    match result {
                        Ok(_) => {}

                        Err(err) => {
                            println!("Error: {:?}", err);

                            stream
                                .write_all(format!("-{err}\r\n").as_bytes())
                                .await
                                .with_context(|| format!("Error writing error to socket: {err:?}"))
                                .unwrap();
                        }
                    }
                });
            }

            Err(e) => {
                println!("error: {e:?}");

                break;
            }
        };
    }

    Ok(())
}

async fn handle_connection(
    _stream: &mut TcpStream,
    service: Arc<RedisService>,
) -> anyhow::Result<()> {
    println!("accepted new connection");

    let mut buffer = [0u8; 2048];

    println!("Reading Data from socket...");

    loop {
        let bytes_read = _stream.read(&mut buffer).await?;

        if bytes_read == 0 {
            break;
        }

        let command =
            String::from_utf8(buffer[0..bytes_read].to_vec()).expect("Could not convert string");

        let response = service.execute_command(&command).await?;

        _stream.write_all(response.as_slice()).await?;
    }

    Ok(())
}
