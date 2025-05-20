use anyhow::Context;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use crate::configs::cmd_options::CmdOptions;
use crate::persistence::rdb::RDB;
use crate::redis_service::RedisService;
use crate::state::server_state::ServerState;

#[derive(Debug)]
pub struct RedisServer {
    state: Arc<RwLock<ServerState>>,
}

impl RedisServer {
    pub fn new(options: CmdOptions) -> Self {
        Self {
            state: Arc::new(RwLock::new(ServerState::from(options))),
        }
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        let mut state = self.state.write().await;

        let address = state.get_address().await;
        let rdb = RDB::new(&state.get_rdb_path().await)?;

        state
            .init_replica()
            .await
            .map_err(|e| {
                println!("Error initializing replica: {e:?}");
            })
            .unwrap_or(());

        // release state lock
        drop(state);

        let service = RedisService::new(self.state.clone(), Box::new(rdb)).await;

        let service_arc = Arc::new(service);

        let listener = TcpListener::bind(address.as_str())
            .await
            .with_context(|| format!("Could not listen on {}", address))
            .unwrap();

        println!("Redis Server started listening on {}", address);

        loop {
            let stream = listener.accept().await;

            match stream {
                Ok((mut stream, _)) => {
                    let service_clone = service_arc.clone();

                    tokio::spawn(async move {
                        println!("accepted new connection");

                        let mut buffer = [0u8; 2048];

                        println!("Reading Data from socket...");

                        while let Ok(bytes_read) = stream.read(&mut buffer).await {
                            println!("Bytes read: {bytes_read}");

                            if bytes_read == 0 {
                                return;
                            }

                            let command = String::from_utf8(buffer[0..bytes_read].to_vec())
                                .expect("Could not convert string");

                            let result = service_clone.execute_command(&command, &mut stream).await;

                            match result {
                                Ok(_) => {}

                                Err(err) => {
                                    println!("Error: {:?}", err);

                                    stream
                                        .write_all(format!("-{err}\r\n").as_bytes())
                                        .await
                                        .with_context(|| {
                                            format!("Error writing error to socket: {err:?}")
                                        })
                                        .unwrap();
                                }
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
}
