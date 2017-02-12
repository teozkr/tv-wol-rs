extern crate libc;
extern crate dbus;

mod tv;
mod cec;
mod avahi;

use std::{net, thread};
use std::sync::mpsc;
use std::io::Read;
use std::time::Duration;

fn init_cec_connection() -> cec::Result<cec::Connection> {
    let mut conn = cec::Connection::new()?;
    conn.init()?;
    Ok(conn)
}

fn init_tv_controller() -> Box<tv::TVController> {
    init_cec_connection().map(|x| Box::new(x) as Box<tv::TVController>).unwrap_or_else(|err| {
        println!("Unable to connect to CEC: {:?}, switching to fake implementation", err);
        Box::new(tv::FakeTVController::new())
    })
}

enum ConnectionEvent {
    Connected,
    Disconnected
}

fn main() {
    let mut controller = init_tv_controller();
    let listener = net::TcpListener::bind("0.0.0.0:0").unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    let entry_group = avahi::register(listener.local_addr().unwrap()).unwrap();

    let (sender, receiver) = mpsc::channel::<ConnectionEvent>();

    thread::spawn(move || {
        let sender = sender;

        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            stream.set_read_timeout(Some(Duration::new(10, 0))).unwrap();
            let conn_sender = sender.clone();
            thread::spawn(move || {
                let sender = conn_sender;
                sender.send(ConnectionEvent::Connected).unwrap();
                let mut buf = [0 as u8; 1024];
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) =>
                            break,
                        _ => {}
                    }
                }
                sender.send(ConnectionEvent::Disconnected).unwrap();
            });
        }
    });

    let mut connections = 0;
    for event in receiver.iter() {
        match event {
            ConnectionEvent::Connected => {
                connections += 1;
                if connections == 1 {
                    controller.turn_on_tv().unwrap();
                }
            },
            ConnectionEvent::Disconnected => {
                connections -= 1;
                if connections == 0 {
                    controller.turn_off_tv().unwrap();
                }
            }
        }
    }
}

