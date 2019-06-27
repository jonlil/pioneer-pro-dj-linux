extern crate tokio_codec;
extern crate bytes;
extern crate futures;
extern crate rand;

use bytes::{Bytes, BytesMut};
use futures::{Future, Async, Poll};
use std::net::{SocketAddr, Ipv4Addr, IpAddr};
use std::io::{Read, Write, self};
use std::thread;
use rand::Rng;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{read_exact, write_all};
use tokio::codec::{BytesCodec, Decoder};
use tokio::prelude::*;

use crate::rpc::server::convert_u16_to_two_u8s_be;
use super::packets::{
    DBMessage,
    DBRequestType,
    DBField,
    DBFieldType,
};
use super::state::{
    LockedClientState as LockedSharedState,
    ClientState as SharedState
};

struct ClientState {
    previous_request: Option<DBRequestType>,
    state: LockedSharedState,
}

impl ClientState {
    pub fn new(state: LockedSharedState) -> Self {
        Self {
            previous_request: None,
            state: state,
        }
    }
}

pub fn client_response(mut stream: TcpStream, data: Bytes) {
    if let Err(e) = stream.write(data.as_ref()) {
        eprintln!("Failed responding to client: {:?}", e);
    }
}

pub enum Event {
    RemoteDBServer,
    Unsupported,
}

pub fn get_package_type(buffer: &[u8]) -> Event {
    match buffer {
         &[0, 0, 0, 15, 82, 101, 109, 111, 116, 101, 68, 66, 83, 101, 114, 118, 101, 114, 0] => Event::RemoteDBServer,
         _ => {
             Event::Unsupported
         }
    }
}

pub fn handle_client(mut stream: TcpStream) {
    let mut buf = [0u8; 64];
    match stream.read(&mut buf) {
        Ok(size) => {
            match get_package_type(&buf[..size]) {
                Event::RemoteDBServer => client_response(
                    stream,
                    Bytes::from(vec![0xff, 0x20]),
                ),
                Event::Unsupported => {},
            }
        },
        Err(err) => eprintln!("{:?}", err),
    }
}

trait Controller {
    fn to_response(&self, request: RequestWrapper, context: &ClientState) -> Bytes;
}

struct RequestWrapper<'a> {
    message: DBMessage<'a>,
}

impl <'a>RequestWrapper<'a> {
    pub fn new(message: DBMessage) -> RequestWrapper {
        RequestWrapper { message: message }
    }

    fn to_response(self) -> BytesMut {
        self.message.to_response()
    }
}

struct RequestHandler<'a> {
    request: RequestWrapper<'a>,
    controller: Box<Controller>,
    context: &'a mut ClientState,
}

impl <'a>RequestHandler<'a> {
    pub fn new(
        request_handler: Box<Controller>,
        message: DBMessage<'a>,
        context: &'a mut ClientState
    ) -> RequestHandler<'a> {
        RequestHandler {
            request: RequestWrapper::new(message),
            controller: request_handler,
            context: context,
        }
    }

    fn respond_to(self) -> Bytes {
        self.controller.to_response(self.request, self.context)
    }
}

fn ok_request() -> Bytes {
    Bytes::from(DBField::new(DBFieldType::U16, &[0x40, 0x00]))
}

struct SetupController;
impl Controller for SetupController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let mut bytes: BytesMut = request.to_response();

        bytes.extend(ok_request());
        bytes.extend(vec![
            0x0f, 0x02,
            0x14, 0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);

        bytes.extend(Bytes::from(DBField::u32(&[0x00, 0x00, 0x00, 0x00])));
        bytes.extend(Bytes::from(DBField::u32(&[0x00, 0x00, 0x00, 0x11])));

        Bytes::from(bytes)
    }
}

struct RootMenuController;
impl Controller for RootMenuController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let mut bytes: BytesMut = request.to_response();

        bytes.extend(ok_request());
        bytes.extend(vec![
            0x0f, 0x02,
            0x14, 0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);

        bytes.extend(Bytes::from(DBField::u32(&[0x00, 0x00, 0x10, 0x00])));
        bytes.extend(Bytes::from(DBField::u32(&[0x00, 0x00, 0x00, 0x08])));

        Bytes::from(bytes)
    }
}

struct ArtistController;
impl Controller for ArtistController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let bytes: BytesMut = request.to_response();

        Bytes::from(bytes)
    }
}

struct RenderController;
impl Controller for RenderController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let bytes: BytesMut = request.to_response();

        Bytes::from(bytes)
    }
}

fn get_controller(request_type: &DBRequestType) -> Option<Box<dyn Controller>> {
    match request_type {
        DBRequestType::ArtistRequest => Some(Box::new(ArtistController)),
        DBRequestType::RenderRequest => Some(Box::new(RenderController)),
        DBRequestType::RootMenuRequest => Some(Box::new(RootMenuController)),
        DBRequestType::Setup => Some(Box::new(SetupController)),
        _ => None,
    }
}

fn process(
    bytes: BytesMut,
    context: &mut ClientState,
    peer_addr: &SocketAddr,
) -> Bytes {
    if bytes.len() == 5 {
        return Bytes::from(bytes);
    }

    match DBMessage::parse(&bytes) {
        Ok((_unprocessed_bytes, message)) => {
            if let Some(request_handler) = get_controller(&message.request_type) {
                let request_type = &message.request_type.clone();
                let bytes = RequestHandler::new(
                    request_handler,
                    message,
                    context,
                ).respond_to();

                context.previous_request = Some(*request_type);

                return bytes;
            } else {
                eprintln!("previous_request: {:?}\nrequest_type => {:?}\narguments => {:#?}\npeer: {:?}\n",
                    context.previous_request,
                    message.request_type,
                    message.args,
                    peer_addr);
            }
        },
        Err(nom::Err::Error((bytes, _))) => {
            eprintln!("Error: {:?}", bytes);
        },
        _ => {
            eprintln!("Not covered: {:?}", bytes);
        },
    }

    Bytes::from("Failed processing request into response")
}

/// Handle library clients
struct LibraryClientHandler;
impl LibraryClientHandler {
    fn spawn(address: SocketAddr, state: LockedSharedState) -> Result<(), io::Error> {
        let listener = TcpListener::bind(&address)?;
        let done = listener
            .incoming()
            .map_err(|err| eprintln!("Failed to accept socket; error = {:?}", err))
            .for_each(move |socket| {
                let peer_addr = socket.peer_addr().unwrap();
                let framed = BytesCodec::new().framed(socket);
                let (writer, reader) = framed.split();
                let mut context = ClientState::new(state.clone());

                let responses = reader.map(move |bytes| {
                    process(bytes, &mut context, &peer_addr)
                });

                let writes = responses.fold(writer, |writer, response| {
                    writer.send(response)
                });

                let processor = writes.then(move |_w| Ok(()));

                tokio::spawn(processor)
            });

        Ok(tokio::run(done))
    }
}

pub struct DBLibraryServer;
impl DBLibraryServer {
    fn spawn(address: &str, state: LockedSharedState) {
        let addr = address.parse::<SocketAddr>().unwrap();
        let listener = TcpListener::bind(&addr).unwrap();
        let done = listener
            .incoming()
            .map_err(|e| println!("failed to accept socket; error = {:?}", e))
            .for_each(move |socket| {
                let port = rand::thread_rng().gen_range(60315, 65315);
                let state = state.clone();
                let allocated_port = port.to_u8_vec();

                let processor = read_exact(socket, vec![0; 19])
                    .and_then(move |(socket, _bytes)| {
                        allocate_library_client_handler(port, state)
                            .then(|_| Ok((socket, allocated_port)))
                    })
                    .and_then(|(socket, allocated_port)| {
                        write_all(socket, allocated_port.to_owned()).then(|_| Ok(()))
                    })
                    .map_err(|err| eprintln!("Failed responding to port: {:?}", err));
                tokio::spawn(processor)
            });
        tokio::run(done);
    }

    pub fn run(state: LockedSharedState) {
        Self::spawn("0.0.0.0:12523", state.clone());
    }
}

trait U16ToVec {
    fn to_u8_vec(self) -> Vec<u8>;
}

impl U16ToVec for u16 {
    fn to_u8_vec(self) -> Vec<u8> {
        convert_u16_to_two_u8s_be(self)
    }
}

fn allocate_library_client_handler(
    port: u16,
    state: LockedSharedState,
) -> InitializeClientLibraryHandler {
    InitializeClientLibraryHandler {
        port: port,
        state: state,
    }
}

struct InitializeClientLibraryHandler {
    port: u16,
    state: LockedSharedState,
}

impl Future for InitializeClientLibraryHandler {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let port = self.port.to_owned();
        let state = self.state.clone();

        thread::spawn(move || {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
            LibraryClientHandler::spawn(addr, state).unwrap();
        });

        Ok(Async::Ready(()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::packets::fixtures;

    pub struct TestController;
    impl Controller for TestController {
        fn to_response(&self, _request: RequestWrapper, _context: &ClientState) -> Bytes {
            Bytes::from("my-very-test-value")
        }
    }

    #[test]
    fn test_controller_trait() {
        let mut context = ClientState::new(SharedState::new());
        let request_handler = RequestHandler::new(
            Box::new(TestController {}),
            fixtures::setup_request_packet().unwrap().1,
            &mut context,
        );

        assert_eq!(request_handler.respond_to(), Bytes::from("my-very-test-value"));
    }

    #[test]
    fn test_setup_request_handling() {
        let mut context = ClientState::new(SharedState::new());
        let request_handler = RequestHandler::new(
            Box::new(SetupController {}),
            fixtures::setup_request_packet().unwrap().1,
            &mut context,
        );

        assert_eq!(request_handler.respond_to(), fixtures::setup_response_packet());
    }

    #[test]
    fn test_root_menu_request_handling() {
        let mut context = ClientState::new(SharedState::new());
        let request_handler = RequestHandler::new(
            Box::new(RootMenuController {}),
            fixtures::root_menu_request().unwrap().1,
            &mut context,
        );

        assert_eq!(request_handler.respond_to(), fixtures::root_menu_response_packet());
    }
}

// TODO: code below will be removed
struct Library;
impl Library {
    fn close_list_item() -> Vec<u8> {
        vec![
            0x11,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x00,
        ]
    }

    fn open_list_item(reference: &(u8, u8))  -> Vec<u8> {
        vec![
            0x11,0x87,0x23,0x49,0xae,
            0x11,0x05,0x80,reference.0,reference.1,
            0x10,0x41,0x01,0x0f,0x0c,0x14,0x00,0x00,
            0x00,0x0c,0x06,0x06,0x06,0x02,
            0x06,0x02,0x06,0x06,0x06,0x06,
            0x06,0x06,
        ]
    }

    // This contains artist and playlists views
    // Seems to be structed data so this will be reusable for listing things in the displays.
    pub fn tbd(reference: (u8, u8)) -> Bytes {
        let mut bytes = Bytes::from(vec![
            0x11,0x87,0x23,0x49,0xae,
            0x11,0x05,0x80,reference.0,reference.1,
            0x10,0x40,0x01,0x0f,0x02,0x14,
            0x00,0x00,0x00,0x0c,0x06,0x06,
            0x00,0x00,0x00,0x00,0x00,
            0x00,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x01,
            0x11,0x00,0x00,0x00,0x00,
        ]);

        bytes.extend(Self::open_list_item(&reference));
        bytes.extend(vec![
            0x11,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x02,
            0x11,0x00,0x00,0x00,0x12,

            0x26,0x00,0x00,0x00,0x09,0xff,0xfa,

            // ARTIST
            0x00,0x41,0x00,0x52,0x00,0x54,0x00,0x49,0x00,0x53,0x00,0x54,

            0xff,0xfb,0x00,0x00,0x11,0x00,0x00,0x00,0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,
            0x00,0x00,0x81,
        ]);
        bytes.extend(Self::close_list_item());

        bytes.extend(Self::open_list_item(&reference));
        bytes.extend(vec![
            // Index??
            0x11,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x03,
            0x11,0x00,0x00,0x00,0x10,

            0x26,0x00,0x00,0x00,0x08,0xff,0xfa,

            // ALBUM
            0x00,0x4a,0x00,0x4f,0x00,0x4e,0x00,0x41,0x00,0x53,

            0xff,0xfb,0x00,0x00,0x11,0x00,0x00,0x00,0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,
            0x00,0x00,0x82,
        ]);
        bytes.extend(Self::close_list_item());

        bytes.extend(Self::open_list_item(&reference));
        bytes.extend(vec![
            // Index???
            0x11,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x04,
            0x11,0x00,0x00,0x00,0x10,

            0x26,0x00,0x00,0x00,0x08,0xff,0xfa,

            // TRACK
            0x00,0x54,0x00,0x52,0x00,0x41,0x00,0x43,0x00,0x4b,

            0xff,0xfb,0x00,0x00,0x11,0x00,0x00,0x00,0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,
            0x00,0x00,0x83,
        ]);
        bytes.extend(Self::close_list_item());

        bytes.extend(Self::open_list_item(&reference));
        bytes.extend(vec![
            0x11,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x0c,
            0x11,0x00,0x00,0x00,0x0c,

            0x26,0x00,0x00,0x00,0x06,0xff,0xfa,

            // KEY
            0x00,0x4b,0x00,0x45,0x00,0x59,

            0xff,0xfb,0x00,0x00,0x11,0x00,0x00,0x00,0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,
            0x00,0x00,0x8b,
        ]);

        bytes.extend(Self::close_list_item());

        bytes.extend(Self::open_list_item(&reference));
        bytes.extend(vec![
            // Index??
            0x11,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x05,
            0x11,0x00,0x00,0x00,0x16,

            0x26,0x00,0x00,0x00,0x0b,0xff,0xfa,

            // PLAYLIST
            0x00,0x50,0x00,0x4c,0x00,0x41,0x00,0x59,0x00,0x4c,0x00,0x49,0x00,0x53,0x00,0x54,

            0xff,0xfb,0x00,0x00,0x11,0x00,0x00,0x00,0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,
            0x00,0x00,0x84,
        ]);
        bytes.extend(Self::close_list_item());

        bytes.extend(Self::open_list_item(&reference));
        bytes.extend(vec![
            // Index?
            0x11,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x16,
            0x11,0x00,0x00,0x00,0x14,

            0x26,0x00,0x00,0x00,0x0a,0xff,0xfa,

            // SOME TEXT
            0x00,0x48,0x00,0x49,0x00,0x53,0x00,0x54,0x00,0x4f,0x00,0x52,0x00,0x59,

            0xff,0xfb,0x00,0x00,0x11,0x00,0x00,0x00,0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,
            0x00,0x00,0x95,
        ]);
        bytes.extend(Self::close_list_item());

        bytes.extend(Self::open_list_item(&reference));
        bytes.extend(vec![
            // Index?
            0x11,0x00,0x00,0x00,0x00,
            0x11,0x00,0x00,0x00,0x12,
            0x11,0x00,0x00,0x00,0x12,

            0x26,0x00,0x00,0x00,0x09,0xff,0xfa,

            // SOME TEXT
            0x00,0x53,0x00,0x45,0x00,0x41,0x00,0x52,0x00,0x43,0x00,0x48,

            0xff,0xfb,0x00,0x00,0x11,0x00,0x00,0x00,0x02,0x26,0x00,0x00,0x00,0x01,0x00,0x00,0x11,0x00,
            0x00,0x00,0x91,
        ]);
        bytes.extend(Self::close_list_item());

        bytes.extend(vec![
            0x11,0x87,0x23,0x49,0xae,
            0x11,0x05,0x80,reference.0,reference.1,
            0x10,0x42,0x01,0x0f,0x00,
            0x14,0x00,0x00,0x00,0x0c,
            0x00,0x00,0x00,0x00,0x00,
            0x00,0x00,0x00,0x00,0x00,
            0x00,0x00,
        ]);

        bytes
    }
}
