#![allow(unused_imports)]
use std::net::TcpListener;
use std::io::{Read, Write};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");

                let mut buffer: Vec<u8> = vec![];

                println!("Reading Data from socket...");

                _stream.read_exact(&mut buffer).expect("Couldn't read");
                
                println!("Buffer: {buffer:?}");

                _stream.write_all(b"+PONG\r\n").expect("Could not write to socket");
            }

            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
