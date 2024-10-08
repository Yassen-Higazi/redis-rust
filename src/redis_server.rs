use std::io::Read;

use anyhow::Context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::resp::RespService;

pub async fn listen() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        let stream = listener.accept().await;

        match stream {
            Ok((mut stream, _)) => {
                tokio::spawn(async move {
                    let result = handle_connection(&mut stream)
                        .await
                        .with_context(|| "Could not handle connection");

                    match result {
                        Ok(_) => {}

                        Err(err) => {
                            println!("Error: {:?}", err);
                            stream
                                .write_all(b"-Server Failure\r\n")
                                .await
                                .with_context(|| format!("Error: {err:?}"))
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

async fn handle_connection(_stream: &mut TcpStream) -> anyhow::Result<()> {
    println!("accepted new connection");

    let mut buffer = [0u8; 512];

    println!("Reading Data from socket...");

    loop {
        let bytes_read = _stream.read(&mut buffer).await?;

        if bytes_read == 0 {
            break;
        }

        let command =
            String::from_utf8(buffer[0..bytes_read].to_vec()).expect("Could not convert string");

        let service = RespService::new();

        let response = service.execute_command(&command).await?;

        _stream.write_all(response.as_slice()).await?;
    }

    Ok(())
}
