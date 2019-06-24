extern crate tokio_codec;
extern crate bytes;
extern crate futures;

use futures::{Future, Async, Poll};
use std::net::{SocketAddr, Ipv4Addr, IpAddr};
use std::io::{Read, Write, self};
use std::thread;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{read_exact, write_all};
use tokio::codec::{BytesCodec, Decoder};
use tokio::prelude::*;
use bytes::{Bytes, BytesMut};

use crate::rpc::server::convert_u16_to_two_u8s_be;
use super::packets::{DBMessage, DBRequestType, DBField, DBFieldType};

struct PlayerState {
    current_page: Option<u8>,
}

struct Library;
impl Library {
    pub fn start_page() -> Vec<u8> {
        vec![0xff, 0x20]
    }

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

pub fn client_response(mut stream: TcpStream, data: Vec<u8>) {
    if let Err(e) = stream.write(data.as_ref()) {
        eprintln!("{:?}", e);
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
                Event::RemoteDBServer => client_response(stream, Library::start_page()),
                Event::Unsupported => {},
            }
        },
        Err(err) => eprintln!("{:?}", err),
    }
}

#[derive(Debug)]
enum Request {
    Initiate(Bytes),
    QueryListItem(Bytes),
    FetchListItemContent(Bytes),
    Unimplemented,
}

trait DBMessageResponseTrait {
    type Item;
    fn to_response(self, request: DBMessage, context: &SharedClientContext) -> Self::Item;
}

struct DBMessageRequest<'a, T: DBMessageResponseTrait> {
    request: DBMessage<'a>,
    response: T,
    context: &'a SharedClientContext,
}

impl<'a, T: DBMessageResponseTrait> DBMessageRequest<'a, T> {
    fn response(self) -> T::Item {
        self.response.to_response(self.request, self.context)
    }
}

struct DBMessageSetupResponse;
impl DBMessageResponseTrait for DBMessageSetupResponse {
    type Item = Bytes;

    fn to_response(self, request: DBMessage, _context: &SharedClientContext) -> Self::Item {
        let mut bytes: BytesMut = request.to_response();

        bytes.extend(vec![0x10, 0x40, 0x00]);
        bytes.extend(vec![
            0x0f, 0x02,
            0x14, 0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]);

        bytes.extend(DBField::new(DBFieldType::U32, &[0x00, 0x00, 0x00, 0x00]).as_bytes());
        bytes.extend(DBField::new(DBFieldType::U32, &[0x00, 0x00, 0x00, 0x11]).as_bytes());

        Bytes::from(bytes)
    }
}

fn is_library_browsing_request(bytes: &[u8]) -> bool {
    bytes == [0x11, 0x87, 0x23, 0x49, 0xae, 0x11]
}

impl Request {
    fn parse(
        input: BytesMut,
        _client_context: &SharedClientContext,
        _player_state: &mut PlayerState
    ) -> Result<Request, &'static str> {
        if input.len() == 5 {
            Ok(Request::Initiate(input.freeze()))
        } else if is_library_browsing_request(&input[0..=5]) {
            Ok(match input.len() {
                47 => {
                    Request::Initiate(Bytes::from(vec![
                        0x11, 0x87, 0x23, 0x49, 0xae, 0x11, 0x05, 0x80,
                        input[8], input[9], 0x10, 0x40, 0x00, 0x0f, 0x02, 0x14,
                        0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                        0x11, 0x00, 0x00, 0x10, 0x00, 0x11, 0x00, 0x00,
                        0x00, 0x08
                    ]))
                },
                42 => {
                    Request::QueryListItem(Bytes::from(vec![
                        0x11, 0x87, 0x23, 0x49, 0xae,
                        0x11, input[6], input[7], input[8], input[9],
                        0x10, 0x40, 0x00,
                        0x0f, 0x02,
                        0x14,
                        0x00, 0x00, 0x00, 0x0c, 0x06, 0x06, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                        0x11, 0x00, 0x00, input[11], input[12],
                        0x11, 0x00, 0x00, 0x00, 0x02,
                    ]))
                },
                62 => {
                    Request::FetchListItemContent(Library::tbd((input[8], input[9])))
                },
                _ => {
                    Request::Unimplemented
                },
            })
        } else {
            eprintln!("parsing TCP package failed; package = {:?}", input);
            Err("parsing TCP package failed")
        }
    }
}


type SharedClientContext = Arc<ClientContext>;
struct ClientContext;

impl ClientContext {
    pub fn new() -> Self {
        Self {}
    }
}

fn process(bytes: BytesMut, client_context: &SharedClientContext, player_state: &mut PlayerState) -> Result<Bytes, &'static str> {
    fn map_request_type_to_response_handler(request_type: &DBRequestType) -> Result<impl DBMessageResponseTrait<Item = Bytes>, &'static str> {
        match request_type {
            DBRequestType::Setup => Ok(DBMessageSetupResponse {}),
            _ => Err("Unhandled"),
        }
    }

    if let Ok((unprocessed_bytes, dbmessage)) = DBMessage::parse(&bytes) {
        // delegate to controller
        if let Ok(response_handler) = map_request_type_to_response_handler(&dbmessage.request_type) {
            return Ok(DBMessageRequest {
                request: dbmessage,
                response: response_handler,
                context: client_context,
            }.response())
        }
    }

    if let Ok(request) = Request::parse(bytes, client_context, player_state) {
        Ok(match request {
            Request::Initiate(response) => response,
            Request::QueryListItem(response) => response,
            Request::FetchListItemContent(response) => response,
            Request::Unimplemented => Bytes::from("Unimplemented"),
        })
    } else {
        Err("Failed processing request into response")
    }
}

/// Handle library clients
struct LibraryClientHandler;

impl LibraryClientHandler {
    fn spawn(address: &SocketAddr, context: SharedClientContext) -> Result<(), io::Error> {
        let listener = TcpListener::bind(address)?;
        let done = listener
            .incoming()
            .map_err(|err| eprintln!("Failed to accept socket; error = {:?}", err))
            .for_each(move |socket| {
                let mut player_state = PlayerState {
                    current_page: None,
                };
                let framed = BytesCodec::new().framed(socket);
                let (writer, reader) = framed.split();
                let context = context.clone();

                let responses = reader.map(move |bytes| {
                    let context = &context;
                    match process(bytes, context, &mut player_state) {
                        Ok(response) => response,
                        Err(err) => Bytes::from(err),
                    }
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
    fn spawn(address: &str, client_context: SharedClientContext) {
        let addr = address.parse::<SocketAddr>().unwrap();
        let listener = TcpListener::bind(&addr).unwrap();

        // TODO: Just use a random port, easier to just let the os manage this.
        let mut tcp_port_pool: Vec<u16> = vec![65312, 65313, 65314, 65315];

        let done = listener
            .incoming()
            .map_err(|e| println!("failed to accept socket; error = {:?}", e))
            .for_each(move |socket| {
                let tcp_port = tcp_port_pool.pop().unwrap();
                let client_context = client_context.clone();
                let allocated_port = tcp_port.to_u8_vec();

                let processor = read_exact(socket, vec![0; 19])
                    .and_then(move |(socket, _bytes)| {
                        allocate_library_client_handler(tcp_port, client_context)
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

    pub fn run() {
        Self::spawn("0.0.0.0:12523", Arc::new(ClientContext::new()));
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

fn allocate_library_client_handler(port: u16, client_context: SharedClientContext) -> InitializeClientLibraryHandler {
    InitializeClientLibraryHandler {
        port: port,
        client_context: client_context
    }
}

struct InitializeClientLibraryHandler {
    port: u16,
    client_context: SharedClientContext,
}

impl Future for InitializeClientLibraryHandler {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let port = self.port.to_owned();
        let client_context = self.client_context.clone();

        thread::spawn(move || {
            LibraryClientHandler::spawn(
                &SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port),
                client_context,
            );
        });

        Ok(Async::Ready(()))
    }
}

#[cfg(test)]
mod test {
    use super::super::packets::fixtures;
    use super::{
        DBMessageRequest,
        DBMessage,
        DBMessageResponseTrait,
        SharedClientContext,
        ClientContext,
        DBMessageSetupResponse,
    };
    use std::sync::Arc;
    use bytes::Bytes;

    pub struct TestDBMessageResponse;
    impl DBMessageResponseTrait for TestDBMessageResponse {
        type Item = Bytes;

        fn to_response(self, request: DBMessage, context: &SharedClientContext) -> Self::Item {
            Bytes::from("my-very-test-value")
        }
    }

    #[test]
    fn test_response_trait() {
        let request_handler = DBMessageRequest {
            request: fixtures::setup_request_packet().unwrap().1,
            response: TestDBMessageResponse {},
            context: &Arc::new(ClientContext::new()),
        };

        assert_eq!(request_handler.response(), Bytes::from("my-very-test-value"));
    }

    #[test]
    fn test_response_to_setup_request() {
        let request_handler = DBMessageRequest {
            request: fixtures::setup_request_packet().unwrap().1,
            response: DBMessageSetupResponse {},
            context: &Arc::new(ClientContext::new()),
        };

        assert_eq!(request_handler.response(), fixtures::setup_response_packet());
    }
}
