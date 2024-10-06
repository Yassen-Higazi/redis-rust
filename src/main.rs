#![allow(unused_imports)]
use std::net::TcpListener;
use std::io::{Read, Write};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");

                let mut buffer = [0u8; 512]; 

                println!("Reading Data from socket...");

                loop {

                    let bytes_read = _stream.read(&mut buffer).expect("Couldn't read");

                    println!("Bytes read: {}", bytes_read);

                    if bytes_read == 0 {
                        break; 
                    }

                    let command = String::from_utf8(buffer[0..bytes_read].to_vec()).expect("Could not convert string");

                    println!("Command: {:?}", command);

                    _stream.write_all(b"+PONG\r\n").expect("Could not write to socket");
                }
            }

            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
