use crate::utils::logging::*;
use crate::utils::utils::*;
use socket2::{Domain, Socket, Type};
use std::io::prelude::*;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::vec;
use std::collections::HashMap;


pub struct Server {
  ip: String,
  port: u16,
  key: String,
  listener: TcpListener,
  server_log: Logger,
}
impl Server {
  pub fn new(ip: String, port: u16, key: String) -> Server {
    let sock = Socket::new(Domain::IPV6, Type::STREAM, None).unwrap();
    sock.set_only_v6(false).unwrap();
    let address: SocketAddr = format!("[{}]:{}", ip, port).parse().unwrap();
    sock.bind(&address.into()).unwrap();
    sock.listen(128).unwrap();
    let listener: TcpListener = sock.into();
    Server {
      ip,
      port,
      key,
      listener,
      server_log: Logger::new(),
    }
  }

  pub fn run_server(&mut self) -> std::io::Result<()> {
    for stream in self.listener.try_clone()?.incoming() {
      match stream {
        Ok(stream) => {
          println!("New connection: {}", stream.peer_addr().unwrap());
          {
            self.handle_client(stream);
          }
        }
        Err(e) => {
          println!("Error: {}", e);
        }
      }
    }
    Ok(())
  }

  /*fn verify_structure(std::vec::Vec<&str>& lines) -> bool {
      // first line must be GET {} HTTP/1.1
      // you should be able to split each line by ": "
      // if you do that you have a pair of strings where the first is the key and the latter is the value
      // You want to see that Host, Upgrade, Connection, Sec-WebSocket-Key, Origin, Sec-WebSocket-Version are
      // all present and each only once
      // Upgrade: websocket, Connection: Upgrade, Sec-WebSocket-Version: 13
      let first_line: Vec<&str> = lines[0].split(" ").collect();
  }*/

  fn verify_client_handshake(stream: &mut TcpStream) -> bool {
    let mut buf = [0; 1024];
    let size = stream.read(&mut buf).unwrap();
    let request = String::from_utf8_lossy(&buf[..size]);
    let lines: std::vec::Vec<&str> = request.split('\n').collect();
    let first_line: vec::Vec<&str> = lines[0].split(' ').collect();
    let last_word = format!(r"{}", first_line[2]);
    if first_line.len() != 3 || first_line[0] != "GET" || 
       !first_line[1].starts_with('/') || last_word.trim() != r"HTTP/1.1" {
      println!("{}", first_line[1].starts_with('/'));
      println!("{}", first_line[0] == "GET");
      println!("{}", first_line[2]);
      println!("early on failure {} second {} third {}", first_line[0], first_line[1], first_line[2]);
      return false;
    }

    let mut m: HashMap<String, String> = HashMap::new();
    for line in lines[1..].iter() {
      let split_line: Vec<&str> = (line.to_owned()).split(": ").collect();
      if split_line.len() == 2 {
        m.insert(String::from(split_line[0]), String::from(split_line[1]));
      }
    }
    let host = m.get("Host").unwrap().to_owned();
    let upgrade = m.get("Upgrade").unwrap().to_owned();
    let connection = m.get("Connection").unwrap().to_owned();
    let key = m.get("Sec-WebSocket-Key").unwrap().to_owned();
    let version = m.get("Sec-WebSocket-Version").unwrap().to_owned();
    let origin = m.get("Origin").unwrap().to_owned();

    if upgrade.trim() != "websocket" || connection.trim() != "Upgrade" || version.trim() != "13" {
      return false;
    }

    let my_key = sec_websocket_key(key);
    let response: String = format!(
      "HTTP/1.1 101 Switching Protocols\n\
      Upgrade: websocket\n\
      Connection: Upgrade\n\
      Sec-WebSocket-Accept: {}",
      my_key
    );
    stream.write(response.as_bytes()).unwrap();
    true
  }

  fn read_message(&mut self, buf: &mut Vec<u8>, stream: &mut TcpStream) -> bool {
    let size = stream.read(buf).unwrap();
    if size == 0 {
      println!("size is 0");
      return false;
    }
    let msg: String = format!("Server Read: {}", String::from_utf8_lossy(&buf[..]));
    let m: Message = Message::new(msg.clone(), ErrorLevel::INFO);
    self.server_log.log(m);
    true
  }

  fn write_message(&mut self, buf: &mut Vec<u8>, stream: &mut TcpStream) -> bool {
    match stream.write(&buf) {
      Ok(_) => {
        let msg: String = format!("Server Write: {}", String::from_utf8_lossy(&buf[..]));
        let m: Message = Message::new(msg.clone(), ErrorLevel::INFO);
        self.server_log.log(m);
        true
      }
      Err(_) => {
        println!(
          "An error occurred while writing, terminating connection with {}",
          stream.peer_addr().unwrap()
        );
        stream.shutdown(Shutdown::Both).unwrap();
        false
      }
    }
  }

  pub fn handle_client(&mut self, mut stream: TcpStream) {
    println!("handling client");
    let mut buf: Vec<u8> = vec![0; 1024];
    let handshake_success: bool = Self::verify_client_handshake(&mut stream);
    if handshake_success {
      while self.read_message(&mut buf, &mut stream) {
        if !self.write_message(&mut buf, &mut stream) {
          break;
        }
      }
    } else {
      println!("Invalid client handshake");
    }
    println!("client all done");
    self.server_log.print_log();
  }
}
