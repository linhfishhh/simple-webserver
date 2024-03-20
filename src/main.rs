use std::{f32::consts::E, io::{self, Read, Write}, net::TcpListener};
enum ConnectionState {
    Read {
        request: [u8; 1024],
        read: usize,
    },
    Write {
        response: &'static [u8],
        written: usize,
    },
    Flush,
}

fn main() {
    // listen tcp on port 3000
    let listener = TcpListener::bind("localhost:3000").unwrap();
    listener.set_nonblocking(true).unwrap();

    let mut conns = Vec::new();

    loop {
        match listener.accept() {
            Ok((connection, _)) => {
                connection.set_nonblocking(true).unwrap();
                let state = ConnectionState::Read {
                    request: [0u8; 1024],
                    read: 0,
                };

                conns.push((connection, state));
            }
            // check error is wouldblock
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("fail to accept request"),
        }

        let mut completed = Vec::new();
        'next: for (i, (connection, state)) in conns.iter_mut().enumerate() {
            if let ConnectionState::Read { request, read } = state {
                loop {
                    match connection.read(&mut request[*read..]) {
                        Ok(0) => {
                            print!("client disconnected");
                            completed.push(i);
                        }
                        Ok(n) => *read += n,
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue 'next,
                        Err(e) => panic!("error: {}", e),
                    }
                    if request.get(*read - 4..*read) == Some(b"\r\n\r\n") {
                        break;
                    }
                }

                let request = String::from_utf8_lossy(&request[..*read]);
                println!("{request}");
                // move into the write state
                let response = concat!(
                    "HTTP/1.1 200 OK\r\n",
                    "Content-Length: 12\n",
                    "Connection: close\r\n\r\n",
                    "Hello world!"
                );

                *state = ConnectionState::Write {
                    response: response.as_bytes(),
                    written: 0,
                };
            }

            if let ConnectionState::Write { response, written } = state {
                loop {
                    match connection.write(&response[*written..]) {
                        Ok(0) =>{
                            print!("client disconnect");
                            completed.push(i);
                            continue 'next;
                        }
                        Ok(n) => *written += n,
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue 'next,
                        Err(e) => panic!("{}", e),
                    }
                    if *written == response.len() {
                        break;  
                    }
                }

                *state = ConnectionState::Flush;
            }

            if let ConnectionState::Flush = state {
                match connection.flush() {
                    Ok(_) => {
                        completed.push(i);
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
          
                        continue 'next;
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        }
        for i in completed.into_iter().rev() {
            conns.remove(i);
        }
    }

 
}