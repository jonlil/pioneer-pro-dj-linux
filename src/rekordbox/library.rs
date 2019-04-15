use std::net::{TcpStream, Shutdown};
use std::io::{self, Read, Write};
use std::thread;
use std::time::Duration;

struct Library;
impl Library {
    pub fn start_page() -> Vec<u8> {
        vec![0xff, 0x20]
    }

    // This contains artist and playlists views
    // Seems to be structed data so this will be reusable for listing things in the displays.
    pub fn tbd() -> Vec<u8> {
        vec![
            0xc8,0x3d,0xfc,0x04,0x1e,0xc4,0xac,0x87,0xa3,0x35,0xbc,0x4d,0x08,0x00,0x45,0x00,0x03,
            0x7e,0x00,0x00,0x40,0x00,0xff,0x06,0x00,0x00,0xa9,0xfe,0x25,0x74,0xa9,0xfe,0x1e,0xc4,
            0xff,0x20,0x04,0x1e,0xa0,0xa2,0xdd,0xe3,0x00,0x00,0x4c,0xdf,0x50,0x18,0xff,0xff,0x9b,
            0xa5,0x00,0x00,0x11,0x87,0x23,0x49,0xae,0x11,0x05,0x80,0x00,0x02,0x10,0x40,0x01,0x0f,
            0x02,0x14,0x00,0x00,0x00,0x0c,0x06,0x06,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
            0x00,0x11,0x00,0x00,0x00,0x01,0x11,0x00,0x00,0x00,0x00,0x11,0x87,0x23,0x49,0xae,0x11,
            0x05,0x80,0x00,0x02,0x10,0x41,0x01,0x0f,0x0c,0x14,0x00,0x00,0x00,0x0c,0x06,0x06,0x06,
            0x02,0x06,0x02,0x06,0x06,0x06,0x06,0x06,0x06,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,
            0x00,0x02,0x11,0x00,0x00,0x00,0x12,0x26,0x00,0x00,0x00,0x09,0xff,0xfa,0x00,0x41,0x00,
            0x52,0x00,0x54,0x00,0x49,0x00,0x53,0x00,0x54,0xff,0xfb,0x00,0x00,0x11,0x00,0x00,0x00,
            0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,0x00,0x00,0x81,0x11,0x00,0x00,0x00,
            0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,
            0x00,0x00,0x00,0x00,0x11,0x87,0x23,0x49,0xae,0x11,0x05,0x80,0x00,0x02,0x10,0x41,0x01,
            0x0f,0x0c,0x14,0x00,0x00,0x00,0x0c,0x06,0x06,0x06,0x02,0x06,0x02,0x06,0x06,0x06,0x06,
            0x06,0x06,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x03,0x11,0x00,0x00,0x00,0x10,
            0x26,0x00,0x00,0x00,0x08,0xff,0xfa,0x00,0x41,0x00,0x4c,0x00,0x42,0x00,0x55,0x00,0x4d,
            0xff,0xfb,0x00,0x00,0x11,0x00,0x00,0x00,0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,
            0x00,0x00,0x00,0x82,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,
            0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x87,0x23,0x49,0xae,
            0x11,0x05,0x80,0x00,0x02,0x10,0x41,0x01,0x0f,0x0c,0x14,0x00,0x00,0x00,0x0c,0x06,0x06,
            0x06,0x02,0x06,0x02,0x06,0x06,0x06,0x06,0x06,0x06,0x11,0x00,0x00,0x00,0x00,0x11,0x00,
            0x00,0x00,0x04,0x11,0x00,0x00,0x00,0x10,0x26,0x00,0x00,0x00,0x08,0xff,0xfa,0x00,0x54,
            0x00,0x52,0x00,0x41,0x00,0x43,0x00,0x4b,0xff,0xfb,0x00,0x00,0x11,0x00,0x00,0x00,0x02,
            0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,0x00,0x00,0x83,0x11,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,
            0x00,0x00,0x00,0x11,0x87,0x23,0x49,0xae,0x11,0x05,0x80,0x00,0x02,0x10,0x41,0x01,0x0f,
            0x0c,0x14,0x00,0x00,0x00,0x0c,0x06,0x06,0x06,0x02,0x06,0x02,0x06,0x06,0x06,0x06,0x06,
            0x06,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x0c,0x11,0x00,0x00,0x00,0x0c,0x26,
            0x00,0x00,0x00,0x06,0xff,0xfa,0x00,0x4b,0x00,0x45,0x00,0x59,0xff,0xfb,0x00,0x00,0x11,
            0x00,0x00,0x00,0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,0x00,0x00,0x8b,0x11,
            0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,
            0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x87,0x23,0x49,0xae,0x11,0x05,0x80,0x00,0x02,
            0x10,0x41,0x01,0x0f,0x0c,0x14,0x00,0x00,0x00,0x0c,0x06,0x06,0x06,0x02,0x06,0x02,0x06,
            0x06,0x06,0x06,0x06,0x06,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x05,0x11,0x00,
            0x00,0x00,0x16,0x26,0x00,0x00,0x00,0x0b,0xff,0xfa,0x00,0x50,0x00,0x4c,0x00,0x41,0x00,
            0x59,0x00,0x4c,0x00,0x49,0x00,0x53,0x00,0x54,0xff,0xfb,0x00,0x00,0x11,0x00,0x00,0x00,
            0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,0x00,0x00,0x84,0x11,0x00,0x00,0x00,
            0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,
            0x00,0x00,0x00,0x00,0x11,0x87,0x23,0x49,0xae,0x11,0x05,0x80,0x00,0x02,0x10,0x41,0x01,
            0x0f,0x0c,0x14,0x00,0x00,0x00,0x0c,0x06,0x06,0x06,0x02,0x06,0x02,0x06,0x06,0x06,0x06,
            0x06,0x06,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x16,0x11,0x00,0x00,0x00,0x14,
            0x26,0x00,0x00,0x00,0x0a,0xff,0xfa,0x00,0x48,0x00,0x49,0x00,0x53,0x00,0x54,0x00,0x4f,
            0x00,0x52,0x00,0x59,0xff,0xfb,0x00,0x00,0x11,0x00,0x00,0x00,0x02,0x26,0x00,0x00,0x00,
            0x01,0x00,0x00,0x11,0x00,0x00,0x00,0x95,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,
            0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,
            0x87,0x23,0x49,0xae,0x11,0x05,0x80,0x00,0x02,0x10,0x41,0x01,0x0f,0x0c,0x14,0x00,0x00,
            0x00,0x0c,0x06,0x06,0x06,0x02,0x06,0x02,0x06,0x06,0x06,0x06,0x06,0x06,0x11,0x00,0x00,
            0x00,0x00,0x11,0x00,0x00,0x00,0x12,0x11,0x00,0x00,0x00,0x12,0x26,0x00,0x00,0x00,0x09,
            0xff,0xfa,0x00,0x53,0x00,0x45,0x00,0x41,0x00,0x52,0x00,0x43,0x00,0x48,0xff,0xfb,0x00,
            0x00,0x11,0x00,0x00,0x00,0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,0x00,0x00,
            0x91,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,
            0x00,0x00,0x00,0x00,0x11,0x00,0x00,0x00,0x00,0x11,0x87,0x23,0x49,0xae,0x11,0x05,0x80,
            0x00,0x02,0x10,0x42,0x01,0x0f,0x00,0x14,0x00,0x00,0x00,0x0c,0x00,0x00,0x00,0x00,0x00,
            0x00,0x00,0x00,0x00,0x00,0x00,0x00,
        ]
    }
}


pub fn handle_client(mut stream: TcpStream) {

    stream.set_nonblocking(true).expect("set_nonblocking call failed");

    let mut buf = vec![];
    loop {
        match stream.read_to_end(&mut buf) {
            Ok(_) => break,
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                // wait until network socket is ready, typically implemented
                // via platform-specific APIs such as epoll or IOCP
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => panic!("encountered IO error: {}", e),
        };
    };

    eprintln!("{:?}", buf);
    if buf == vec![0, 0, 0, 15, 82, 101, 109, 111, 116, 101, 68, 66, 83, 101, 114, 118, 101, 114, 0] {
        eprintln!("Sending start page payload, {:?}", String::from_utf8_lossy(&[0xff, 0x20]));
        stream.write(Library::start_page().as_ref());
    }
}
