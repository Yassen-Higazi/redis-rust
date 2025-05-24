use anyhow::Context;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};
use tokio::time::timeout;

use crate::configs::cmd_options::CmdOptions;
use crate::persistence::rdb::RDB;
use crate::redis_service::RedisService;
use crate::state::server_state::ServerState;

#[derive(Debug)]
pub struct RedisServer {
    service: Arc<RedisService>,
    state: Arc<RwLock<ServerState>>,
}

impl RedisServer {
    pub fn new(options: CmdOptions) -> Self {
        let state = ServerState::from(options);

        let rdb = RDB::new(&state.get_rdb_path()).expect("Could not create RDB instance");

        let final_state = Arc::new(RwLock::new(state));

        let service = Arc::new(RedisService::new(final_state.clone(), Box::new(rdb)));

        Self {
            service,
            state: final_state,
        }
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        let mut state = self.state.write().await;

        let address = state.get_address();

        let listener = TcpListener::bind(address.as_str())
            .await
            .with_context(|| format!("Could not listen on {}", address))
            .unwrap();

        println!("Redis Server started listening on {}", address);

        let result = state.init_replica().await.map_err(|e| {
            println!("Error initializing replica: {e:?}");
        });

        match result {
            Ok(connection) => {
                println!("Replica initialized successfully");

                self.handle_connection(connection, false);
            }
            Err(e) => {
                eprintln!("Error initializing replica: {e:?}");
            }
        }

        drop(state);

        loop {
            let stream = listener.accept().await;

            match stream {
                Ok((stream, _)) => {
                    let stream_arc = Arc::new(Mutex::new(stream));
                    self.handle_connection(stream_arc, true);
                }

                Err(e) => {
                    eprintln!("error: {e:?}");
                }
            };
        }
    }

    fn handle_connection(&self, stream_arc: Arc<Mutex<TcpStream>>, with_timeout: bool) {
        let service_clone = self.service.clone();

        tokio::spawn(async move {
            println!("accepted new connection");

            let mut buffer = [0u8; 2048];

            let mut stream_guard = stream_arc.lock().await;

            let mut res = stream_guard.read(&mut buffer).await;

            drop(stream_guard);

            while let Ok(bytes_read) = res {
                if bytes_read == 0 {
                    break;
                }

                service_clone
                    .execute_command(&buffer[0..bytes_read], stream_arc.clone())
                    .await
                    .unwrap();

                let mut stream_guard = stream_arc.lock().await;

                let address = stream_guard
                    .peer_addr()
                    .unwrap_or_else(|_| "unknown address".parse().unwrap());

                // set timeout for reading from the stream to not block when the client
                // is not sending data
                res = if with_timeout {
                    match timeout(Duration::from_secs(1), stream_guard.read(&mut buffer)).await {
                        Ok(Ok(bytes_read)) => Ok(bytes_read),

                        Ok(Err(e)) => {
                            eprintln!("error reading from stream: {e:?}");
                            break;
                        }

                        Err(_) => {
                            eprintln!("timeout reading from stream {address}");
                            break;
                        }
                    }
                } else {
                    stream_guard.read(&mut buffer).await
                };
            }
        });
    }
}
