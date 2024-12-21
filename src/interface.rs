use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};

pub trait LinkCable {
    fn send(&mut self, data: u8);
    fn try_recv(&mut self) -> Option<u8>;
}

pub struct NetworkCable {
    client_tx: Sender<u8>,
    server_rx: Receiver<u8>,
    buffer: u8,
}

impl LinkCable for NetworkCable {
    fn send(&mut self, data: u8) {
        self.client_tx.send(data).unwrap();
    }

    fn try_recv(&mut self) -> Option<u8> {
        match self.server_rx.try_recv() {
            Ok(data) => {
                println!("受信データ ◯: {}", data);
                // self.buffer = data;
                Some(data)
            }
            Err(_) => {
                // println!("受信データ ×: None");
                // Some(self.buffer)
                None
            }
        }
    }
}

impl NetworkCable {
    pub fn new(listen_port: String, send_port: String) -> Self {
        let (server_tx, server_rx): (Sender<u8>, Receiver<u8>) = channel();
        let (client_tx, client_rx): (Sender<u8>, Receiver<u8>) = channel();
        std::thread::spawn(move || {
            NetworkCable::create_server(listen_port.clone(), server_tx);
        });
        std::thread::spawn(move || {
            NetworkCable::create_client(send_port, client_rx);
        });

        NetworkCable {
            client_tx,
            server_rx,
            buffer: 0xFF,
        }
    }

    fn create_server(listen_port: String, main_tx: Sender<u8>) {
        // listen_portで待ち受ける
        // 接続がある度に処理スレッドを作成
        let listener = TcpListener::bind(format!("127.0.0.1:{listen_port}")).unwrap();

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let tx = main_tx.clone();
                    std::thread::spawn(move || {
                        NetworkCable::handle_client(&mut stream, tx);
                    });
                }
                Err(e) => {
                    println!("failed to accept socket; error = {:?}", e);
                }
            }
        }
    }

    fn handle_client(stream: &mut TcpStream, tx: Sender<u8>) {
        let mut buffer = [0];
        // let mut buffer = Vec::new();
        loop {
            match stream.read(&mut buffer) {
                // match stream.read_to_end(&mut buffer) {
                Ok(0) => {
                    println!("client disconnected");
                    break;
                }
                Ok(n) => {
                    let data = buffer[..n].to_vec();
                    // bufferの最後のu8
                    println!("受信データ: {:?}", buffer);
                    println!("長さ: {}", n);
                    // let data = buffer[n - 1];
                    tx.send(data[n - 1]).unwrap();
                    // tx.send(data).unwrap();
                }
                Err(e) => {
                    println!("failed to read from socket; error = {:?}", e);
                    break;
                }
            }
        }
    }

    fn create_client(send_port: String, client_rx: Receiver<u8>) {
        let server_addr = format!("127.0.0.1:{send_port}");
        std::thread::spawn(move || {
            let mut client = Client::new(server_addr, client_rx);
            loop {
                match client.client_rx.recv() {
                    Ok(data) => {
                        client.send(data);
                    }
                    Err(e) => {
                        println!("failed to receive data; error = {:?}", e);
                        break;
                    }
                }
            }
        });
    }
}

struct Client {
    stream: Option<TcpStream>,
    server_addr: String,
    client_rx: Receiver<u8>,
}

impl Client {
    fn new(server_addr: String, client_rx: Receiver<u8>) -> Self {
        // let stream = TcpStream::connect(&server_addr).unwrap();
        Client {
            stream: None,
            server_addr,
            client_rx,
        }
    }

    fn send(&mut self, data: u8) {
        self.ensure_connection();
        if let Some(ref mut stream) = self.stream {
            match stream.write_all(&[data]) {
                // Ok(_) => println!("データを送信しました: {}", data),
                Ok(_) => {}
                Err(e) => {
                    // println!("データの送信に失敗しました: {}", e);
                    self.stream = None;
                }
            }
        } else {
            // println!("サーバーへの接続が確立されていません。")
        }
    }

    fn ensure_connection(&mut self) {
        if self.stream.is_none() {
            match TcpStream::connect(&self.server_addr) {
                Ok(stream) => {
                    // println!("サーバに接続しました：{}", self.server_addr);
                    stream
                        .set_write_timeout(Some(std::time::Duration::from_secs(5)))
                        .unwrap();

                    self.stream = Some(stream);
                }
                Err(e) => {
                    // println!("サーバーへの接続に失敗しました。 {:?}", e);
                }
            }
        }
    }
}
