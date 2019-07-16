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

use super::packets::DBMessage;
use super::db_field::{DBField, DBFieldType};
use super::db_request_type::DBRequestType;
use super::db_message_argument::ArgumentCollection;
use super::metadata_type;
use super::state::{
    LockedClientState as LockedSharedState,
    ClientState as SharedState,
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

struct RequestWrapper {
    message: DBMessage,
}

impl RequestWrapper {
    pub fn new(message: DBMessage) -> RequestWrapper {
        RequestWrapper { message: message }
    }

    fn to_response(self) -> BytesMut {
        self.message.to_response()
    }
}

struct RequestHandler<'a> {
    request: RequestWrapper,
    controller: Box<Controller>,
    context: &'a mut ClientState,
}

impl <'a>RequestHandler<'a> {
    pub fn new(
        request_handler: Box<Controller>,
        message: DBMessage,
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
        bytes.extend(Bytes::from(ArgumentCollection::new(vec![
            DBField::from([0x00, 0x00, 0x00, 0x00]),
            DBField::from([0x00, 0x00, 0x00, 0x11]),
        ])));

        Bytes::from(bytes)
    }
}

struct RootMenuController;
impl Controller for RootMenuController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let mut bytes: BytesMut = request.to_response();

        bytes.extend(ok_request());
        bytes.extend(Bytes::from(
            ArgumentCollection::new(vec![
                DBField::from([0x00, 0x00, 0x10, 0x00]),
                DBField::from([0x00, 0x00, 0x00, 0x08]),
            ]),
        ));

        Bytes::from(bytes)
    }
}

struct NavigationController;
impl Controller for NavigationController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let request_type = request.message.request_type;
        let mut bytes: BytesMut = request.to_response();
        let request_type_value = request_type.value();
        let items_to_render: u32 = 1u32;

        bytes.extend(ok_request());
        bytes.extend(Bytes::from(
            ArgumentCollection::new(vec![
                DBField::from([0u8, 0u8, request_type_value[0], request_type_value[1]]),
                DBField::from(items_to_render),
            ]),
        ));

        Bytes::from(bytes)
    }
}

struct PreviewWaveformController;
impl Controller for PreviewWaveformController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let mut bytes: BytesMut = request.to_response();
        bytes.extend(Bytes::from(DBField::from([0x44, 0x02])));
        bytes.extend(Bytes::from(ArgumentCollection::new(vec![
            DBField::from([0x00, 0x00, 0x20, 0x04]),
            DBField::from(0u32),
            DBField::from([0x00, 0x00, 0x03, 0x88]),
            DBField::new(DBFieldType::Binary, &[
                0x18, 0x00, 0x15, 0x00,
                0x16, 0x00, 0x18, 0x00, 0x14, 0x00, 0x0d, 0x00,
                0x18, 0x00, 0x15, 0x00, 0x17, 0x00, 0x17, 0x00,
                0x15, 0x00, 0x17, 0x00, 0x18, 0x00, 0x17, 0x00,
                0x18, 0x00, 0x17, 0x00, 0x15, 0x01, 0x0d, 0x01,
                0x17, 0x00, 0x17, 0x00, 0x17, 0x00, 0x17, 0x00,
                0x17, 0x00, 0x15, 0x00, 0x18, 0x00, 0x15, 0x00,
                0x16, 0x00, 0x18, 0x00, 0x11, 0x01, 0x11, 0x01,
                0x17, 0x00, 0x17, 0x00, 0x15, 0x00, 0x17, 0x00,
                0x18, 0x00, 0x16, 0x00, 0x15, 0x00, 0x15, 0x00,
                0x16, 0x03, 0x15, 0x03, 0x12, 0x03, 0x13, 0x03,
                0x0b, 0x05, 0x0f, 0x05, 0x14, 0x03, 0x12, 0x03,
                0x12, 0x03, 0x12, 0x03, 0x13, 0x02, 0x13, 0x02,
                0x13, 0x03, 0x15, 0x03, 0x16, 0x03, 0x12, 0x03,
                0x0f, 0x05, 0x08, 0x05, 0x15, 0x03, 0x14, 0x03,
                0x12, 0x03, 0x12, 0x03, 0x13, 0x00, 0x16, 0x00,
                0x12, 0x03, 0x13, 0x03, 0x15, 0x03, 0x13, 0x03,
                0x12, 0x03, 0x13, 0x03, 0x08, 0x05, 0x0e, 0x05,
                0x16, 0x03, 0x12, 0x03, 0x12, 0x02, 0x13, 0x02,
                0x16, 0x03, 0x0d, 0x03, 0x13, 0x03, 0x15, 0x03,
                0x0e, 0x05, 0x10, 0x05, 0x12, 0x03, 0x13, 0x03,
                0x13, 0x03, 0x13, 0x03, 0x12, 0x03, 0x12, 0x03,
                0x13, 0x00, 0x16, 0x00, 0x12, 0x03, 0x13, 0x03,
                0x12, 0x05, 0x08, 0x05, 0x13, 0x03, 0x12, 0x03,
                0x13, 0x03, 0x14, 0x03, 0x16, 0x03, 0x0d, 0x03,
                0x13, 0x02, 0x15, 0x02, 0x10, 0x03, 0x15, 0x03,
                0x13, 0x03, 0x15, 0x03, 0x09, 0x05, 0x12, 0x05,
                0x12, 0x03, 0x13, 0x03, 0x13, 0x03, 0x13, 0x03,
                0x0d, 0x02, 0x13, 0x02, 0x15, 0x03, 0x14, 0x03,
                0x13, 0x03, 0x13, 0x03, 0x0b, 0x05, 0x0f, 0x05,
                0x13, 0x03, 0x0f, 0x03, 0x12, 0x03, 0x16, 0x03,
                0x13, 0x02, 0x12, 0x02, 0x13, 0x03, 0x15, 0x03,
                0x10, 0x03, 0x14, 0x03, 0x0f, 0x05, 0x0b, 0x05,
                0x15, 0x03, 0x13, 0x03, 0x0f, 0x03, 0x12, 0x03,
                0x16, 0x03, 0x13, 0x03, 0x0e, 0x02, 0x13, 0x02,
                0x15, 0x03, 0x16, 0x03, 0x0d, 0x02, 0x11, 0x02,
                0x05, 0x05, 0x0f, 0x05, 0x11, 0x03, 0x0f, 0x03,
                0x12, 0x03, 0x0f, 0x03, 0x13, 0x02, 0x0e, 0x02,
                0x12, 0x03, 0x12, 0x03, 0x0e, 0x03, 0x12, 0x03,
                0x0e, 0x05, 0x09, 0x05, 0x11, 0x03, 0x14, 0x03,
                0x12, 0x03, 0x12, 0x03, 0x0e, 0x02, 0x14, 0x02,
                0x0d, 0x03, 0x11, 0x03, 0x13, 0x03, 0x13, 0x03,
                0x04, 0x05, 0x0c, 0x05, 0x10, 0x03, 0x12, 0x03,
                0x11, 0x03, 0x0f, 0x03, 0x14, 0x02, 0x15, 0x02,
                0x12, 0x02, 0x0a, 0x02, 0x11, 0x03, 0x12, 0x03,
                0x06, 0x05, 0x0e, 0x05, 0x10, 0x03, 0x10, 0x03,
                0x12, 0x03, 0x12, 0x03, 0x0e, 0x02, 0x11, 0x02,
                0x10, 0x03, 0x14, 0x03, 0x0a, 0x03, 0x13, 0x03,
                0x0a, 0x03, 0x11, 0x03, 0x10, 0x03, 0x13, 0x03,
                0x10, 0x03, 0x11, 0x03, 0x12, 0x03, 0x0d, 0x03,
                0x12, 0x02, 0x12, 0x02, 0x13, 0x03, 0x0e, 0x03,
                0x0d, 0x05, 0x06, 0x05, 0x12, 0x03, 0x0f, 0x03,
                0x0d, 0x03, 0x15, 0x03, 0x10, 0x03, 0x0a, 0x03,
                0x03, 0x05, 0x04, 0x05, 0x02, 0x05, 0x03, 0x05,
                0x02, 0x05, 0x02, 0x05, 0x02, 0x05, 0x0e, 0x05,
                0x02, 0x05, 0x02, 0x05, 0x02, 0x05, 0x05, 0x05,
                0x02, 0x05, 0x02, 0x05, 0x02, 0x05, 0x02, 0x05,
                0x0b, 0x05, 0x04, 0x05, 0x04, 0x05, 0x03, 0x05,
                0x03, 0x05, 0x02, 0x05, 0x02, 0x05, 0x02, 0x05,
                0x06, 0x05, 0x09, 0x05, 0x02, 0x05, 0x02, 0x05,
                0x02, 0x05, 0x05, 0x05, 0x02, 0x05, 0x02, 0x05,
                0x02, 0x05, 0x0a, 0x05, 0x04, 0x05, 0x04, 0x05,
                0x03, 0x05, 0x03, 0x05, 0x02, 0x05, 0x02, 0x05,
                0x02, 0x05, 0x04, 0x05, 0x08, 0x05, 0x02, 0x05,
                0x02, 0x05, 0x02, 0x05, 0x05, 0x05, 0x02, 0x05,
                0x02, 0x05, 0x02, 0x05, 0x05, 0x05, 0x06, 0x05,
                0x02, 0x05, 0x02, 0x05, 0x02, 0x05, 0x04, 0x05,
                0x02, 0x05, 0x02, 0x05, 0x02, 0x05, 0x0a, 0x05,
                0x02, 0x05, 0x02, 0x05, 0x02, 0x05, 0x05, 0x05,
                0x02, 0x05, 0x02, 0x05, 0x02, 0x05, 0x02, 0x05,
                0x15, 0x02, 0x13, 0x02, 0x14, 0x03, 0x15, 0x03,
                0x13, 0x03, 0x15, 0x03, 0x14, 0x02, 0x12, 0x02,
                0x10, 0x05, 0x0f, 0x05, 0x17, 0x02, 0x17, 0x02,
                0x0b, 0x05, 0x11, 0x05, 0x15, 0x03, 0x11, 0x03,
                0x17, 0x03, 0x13, 0x03, 0x16, 0x03, 0x14, 0x03,
                0x16, 0x00, 0x15, 0x00, 0x14, 0x05, 0x0c, 0x05,
                0x15, 0x03, 0x15, 0x03, 0x15, 0x03, 0x15, 0x03,
                0x0f, 0x03, 0x15, 0x03, 0x16, 0x03, 0x15, 0x03,
                0x11, 0x02, 0x16, 0x02, 0x12, 0x05, 0x10, 0x05,
                0x17, 0x02, 0x17, 0x02, 0x09, 0x05, 0x13, 0x05,
                0x12, 0x03, 0x16, 0x03, 0x15, 0x03, 0x14, 0x03,
                0x16, 0x02, 0x14, 0x02, 0x15, 0x03, 0x15, 0x03,
                0x11, 0x05, 0x0c, 0x05, 0x12, 0x03, 0x16, 0x03,
                0x14, 0x03, 0x13, 0x03, 0x14, 0x03, 0x15, 0x03,
                0x0f, 0x00, 0x16, 0x00, 0x0c, 0x03, 0x12, 0x03,
                0x17, 0x02, 0x17, 0x02, 0x0a, 0x03, 0x11, 0x03,
                0x15, 0x03, 0x10, 0x03, 0x16, 0x03, 0x14, 0x03,
                0x15, 0x02, 0x13, 0x02, 0x15, 0x05, 0x08, 0x05,
                0x17, 0x02, 0x17, 0x02, 0x0a, 0x05, 0x10, 0x05,
                0x14, 0x03, 0x17, 0x03, 0x11, 0x03, 0x15, 0x03,
                0x14, 0x03, 0x15, 0x03, 0x14, 0x00, 0x17, 0x00,
                0x11, 0x03, 0x16, 0x03, 0x12, 0x05, 0x0a, 0x05,
                0x16, 0x03, 0x14, 0x03, 0x11, 0x03, 0x14, 0x03,
                0x15, 0x03, 0x16, 0x03, 0x14, 0x02, 0x10, 0x02,
                0x16, 0x03, 0x14, 0x03, 0x15, 0x03, 0x14, 0x03,
                0x0a, 0x05, 0x11, 0x05, 0x0f, 0x03, 0x17, 0x03,
                0x0d, 0x04, 0x05, 0x04, 0x02, 0x05, 0x02, 0x05,
                0x02, 0x05, 0x02, 0x05, 0x02, 0x05, 0x02, 0x05,
                0x02, 0x05, 0x02, 0x05, 0x02, 0x05, 0x02, 0x05,
                0x02, 0x05, 0x02, 0x05, 0x02, 0x05, 0x02, 0x05,
                0x02, 0x03, 0x02, 0x05, 0x0e, 0x0e, 0x0f, 0x0f,
                0x0e, 0x0f, 0x0f, 0x0e, 0x0f, 0x0e, 0x0c, 0x0e,
                0x0e, 0x0c, 0x0e, 0x0e, 0x0c, 0x0e, 0x0e, 0x0c,
                0x0e, 0x0e, 0x0c, 0x0e, 0x0e, 0x0c, 0x0e, 0x0e,
                0x0c, 0x0e, 0x0e, 0x0c, 0x0e, 0x0c, 0x0e, 0x0d,
                0x0d, 0x0c, 0x0e, 0x0e, 0x0c, 0x0d, 0x0e, 0x0d,
                0x0d, 0x0d, 0x0c, 0x0e, 0x0d, 0x0c, 0x0d, 0x0e,
                0x08, 0x02, 0x07, 0x04, 0x05, 0x02, 0x07, 0x04,
                0x02, 0x05, 0x02, 0x07, 0x04, 0x05, 0x04, 0x05,
                0x05, 0x05, 0x0e, 0x0e, 0x0e, 0x0d, 0x0e, 0x0d,
                0x0e, 0x0e, 0x0e, 0x0c, 0x0e, 0x0e, 0x0c, 0x0e,
                0x0e, 0x0d, 0x0e, 0x0d, 0x0e, 0x0e, 0x0e, 0x0c,
                0x0e, 0x0e, 0x0d, 0x0e, 0x02, 0x01, 0x01, 0x01,
                0x9e, 0xeb, 0x78, 0x10
            ]),
        ])));

        Bytes::from(bytes)
    }
}

struct TitleController;
impl Controller for TitleController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let mut bytes: BytesMut = request.to_response();

        Bytes::from(bytes)
    }
}

struct Response {
    buffer: BytesMut,
}

impl Response {
    pub fn new() -> Response {
        Response {
            buffer: BytesMut::new(),
        }
    }

    fn extend_items(&mut self, items: Vec<Bytes>) {
        for item in items {
            self.extend(item)
        }
    }

    fn extend(&mut self, item: Bytes) {
        self.buffer.extend(item)
    }
}

impl From<Response> for Bytes {
    fn from(response: Response) -> Bytes {
        Bytes::from(response.buffer)
    }
}

fn build_message_header(transaction_id: &DBField) -> DBMessage {
    DBMessage::new(
        transaction_id.clone(),
        DBRequestType::MenuHeader,
        ArgumentCollection::new(vec![
            DBField::from([0x00, 0x00, 0x00, 0x01]),
            DBField::from([0x00, 0x00, 0x00, 0x00]),
        ]),
    )
}

fn build_message_item(transaction_id: &DBField, metadata: (DBField, u32, u8, u8, u8)) -> DBMessage {
    DBMessage::new(
        transaction_id.clone(),
        DBRequestType::MenuItem,
        ArgumentCollection::new(vec![
            DBField::from([0x00, 0x00, 0x00, 0x00]),
            DBField::from([0x00, 0x00, 0x00, metadata.2]),
            DBField::from([0x00, 0x00, 0x00, metadata.3]),
            metadata.0,
            DBField::from([0x00, 0x00, 0x00, 0x02]),
            DBField::from(""),
            DBField::from(metadata.1),
            DBField::from([0x00, 0x00, 0x00, 0x00]),
            DBField::from([0x00, 0x00, 0x00, 0x00]),
            DBField::from([0x00, 0x00, 0x00, 0x00]),
            DBField::from([0x00, 0x00, metadata.4, 0x00]),
            DBField::from([0x00, 0x00, 0x00, 0x00]),
        ]),
    )
}

fn build_message_footer(transaction_id: &DBField) -> DBMessage {
    DBMessage::new(
        transaction_id.clone(),
        DBRequestType::MenuFooter,
        ArgumentCollection::new(vec![
            DBField::from(1u32),
            DBField::from(1u32),
        ]),
    )
}

struct RenderController;
impl RenderController {
    fn render_root_menu(&self, request: RequestWrapper, mut response: Response, context: &ClientState) -> Response {
        let transaction_id = request.message.transaction_id.clone();

        response.extend(Bytes::from(build_message_header(&transaction_id)));
        response.extend_items(vec![
                // MenuName, MetadataType, MenuId
                ("\u{fffa}ARTIST\u{fffb}", metadata_type::ROOT_ARTIST, 0x02, 0x12),
                ("\u{fffa}ALBUM\u{fffb}", metadata_type::ROOT_ALBUM, 0x03, 0x10),
                ("\u{fffa}TRACK\u{fffb}", metadata_type::ROOT_TRACK, 0x04, 0x10),
                ("\u{fffa}KEY\u{fffb}", metadata_type::ROOT_KEY, 0x0c, 0x0c),
                ("\u{fffa}PLAYLIST\u{fffb}", metadata_type::ROOT_PLAYLIST, 0x05, 0x16),
                ("\u{fffa}HISTORY\u{fffb}", metadata_type::ROOT_HISTORY, 0x16, 0x14),
                ("\u{fffa}SEARCH\u{fffb}", metadata_type::ROOT_SEARCH, 0x12, 0x12),
            ].iter().map(|item| {
                Bytes::from(build_message_item(&transaction_id, (
                    DBField::from(item.0),
                    item.1,
                    item.2,
                    item.3,
                    0x00,
                )))
            }).collect());
        response.extend(Bytes::from(build_message_footer(&transaction_id)));

        response
    }

    fn render_artist_page(&self, request: RequestWrapper, mut response: Response, _context: &ClientState) -> Response {
        let transaction_id = request.message.transaction_id;

        response.extend(Bytes::from(build_message_header(&transaction_id)));
        response.extend_items(vec![
            Bytes::from(build_message_item(&transaction_id, (
                DBField::from("Loopmasters"),
                metadata_type::ARTIST,
                0x01,
                0x18,
                0x00,
            )))
        ]);
        response.extend(Bytes::from(DBMessage::new(
            transaction_id,
            DBRequestType::MenuFooter,
            ArgumentCollection::new(vec![]),
        )));

        response
    }

    fn render_title_page(&self, request: RequestWrapper, mut response: Response, _context: &ClientState) -> Response {
        let transaction_id = request.message.transaction_id;

        response.extend(Bytes::from(build_message_header(&transaction_id)));
        response.extend_items(vec![
            Bytes::from(build_message_item(&transaction_id, (
                DBField::from("Loopmasters"),
                metadata_type::TITLE,
                0x05,
                0x1a,
                0x00
            )))
        ]);
        response.extend(Bytes::from(DBMessage::new(
            transaction_id,
            DBRequestType::MenuFooter,
            ArgumentCollection::new(vec![]),
        )));

        response
    }

    fn render_album_by_artist(&self, request: RequestWrapper, mut response: Response, _context: &ClientState) -> Response {
        let transaction_id = request.message.transaction_id;

        response.extend(Bytes::from(build_message_header(&transaction_id)));
        response.extend_items(vec![
            Bytes::from(build_message_item(&transaction_id, (
                DBField::from("Unknown"),
                metadata_type::ALBUM,
                0x00,
                0x10,
                0x00,
            )))
        ]);
        response.extend(Bytes::from(DBMessage::new(
            transaction_id,
            DBRequestType::MenuFooter,
            ArgumentCollection::new(vec![]),
        )));
        response
    }

    fn render_title_by_artist_album(&self, request: RequestWrapper, mut response: Response, _context: &ClientState) -> Response {
        let transaction_id = request.message.transaction_id;

        response.extend(Bytes::from(build_message_header(&transaction_id)));
        let mut items: Vec<(&str, u8, u8)> = vec![("Demo Track 1", 0x05, 0x1a)];

        // This seems to be related to only query one MenuItem
        if request.message.arguments[2 as usize].value > Bytes::from(vec![0x00, 0x00, 0x00, 0x01]) {
            items.extend(vec![("Demo Track 2", 0x06, 0x1a)]);
        }

        response.extend_items(items.iter().map(|item| {
            Bytes::from(build_message_item(&transaction_id, (
                DBField::from(item.0),
                metadata_type::TITLE,
                item.1,
                item.2,
                0x00
            )))
        }).collect());

        response.extend(Bytes::from(DBMessage::new(
            transaction_id,
            DBRequestType::MenuFooter,
            ArgumentCollection::new(vec![]),
        )));
        response
    }

    fn render_metadata(&self, request: RequestWrapper, mut response: Response, _context: &ClientState) -> Response {
        let transaction_id = request.message.transaction_id;

        response.extend(Bytes::from(build_message_header(&transaction_id)));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x01]),
                    DBField::from([0x00, 0x00, 0x00, 0x05]),
                    DBField::from([0x00, 0x00, 0x00, 0x1a]),
                    DBField::from("Demo Track 1"),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::TITLE),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x01, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ]),
            )
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x01]),
                    DBField::from([0x00, 0x00, 0x00, 0x01]),
                    DBField::from([0x00, 0x00, 0x00, 0x18]),
                    DBField::from("Loopmasters"),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::ARTIST),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            )
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x01]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::ALBUM),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ]),
            ),
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0xac]),  // 172, DURATION in seconds?
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::DURATION),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            ),
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x32, 0x00]),  // <- 12800, BPM VALUE?
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::BPM),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            ),
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x05]),
                    DBField::from([0x00, 0x00, 0x00, 0x3c]),
                    DBField::from("Tracks by www.loopmasters.com"),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::COMMENT),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            ),
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x01]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::KEY),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            ),
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::RATING),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            ),
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::COLOR_NONE),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            ),
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::GENRE),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            ),
        ));
        response.extend(Bytes::from(DBMessage::new(
            transaction_id,
            DBRequestType::MenuFooter,
            ArgumentCollection::new(vec![]),
        )));

        response
    }

    fn render_mount_info(&self, request: RequestWrapper, mut response: Response, _context: &ClientState) -> Response {
        let transaction_id = request.message.transaction_id;

        response.extend(Bytes::from(build_message_header(&transaction_id)));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x01]),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::TITLE),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x01, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            ),
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0xac]),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::DURATION),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            ),
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x32, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::BPM),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            ),
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x3c]),
                    DBField::from("Tracks by www.loopmasters.com"),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from(metadata_type::COMMENT),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ]),
            ),
        ));
        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x69, 0x47, 0xa8]),
                    DBField::from([0x00, 0x00, 0x00, 0x05]),
                    DBField::from([0x00, 0x00, 0x00, 0x7a]),
                    DBField::from("C:/Users/Snaajf/Music/PioneerDJ/Demo Tracks/Demo Track 1.mp3"),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            ),
        ));

        response.extend(Bytes::from(
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                ArgumentCollection::new(vec![
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x01]),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x02]),
                    DBField::from(""),
                    DBField::from([0x00, 0x00, 0x00, 0x2f]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                    DBField::from([0x00, 0x00, 0x00, 0x00]),
                ])
            ),
        ));

        response.extend(Bytes::from(DBMessage::new(
            transaction_id,
            DBRequestType::MenuFooter,
            ArgumentCollection::new(vec![]),
        )));

        response
    }
}

struct QueryMountInfoController;
impl Controller for QueryMountInfoController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let request_type = request.message.request_type;
        let mut bytes: BytesMut = request.to_response();
        let request_type_value = request_type.value();
        let items_to_render: u32 = 6u32;

        bytes.extend(ok_request());
        bytes.extend(Bytes::from(
            ArgumentCollection::new(vec![
                DBField::from([0u8, 0u8, request_type_value[0], request_type_value[1]]),
                DBField::from(items_to_render),
            ]),
        ));

        Bytes::from(bytes)
    }
}

struct TitleByArtistAlbumController;
impl Controller for TitleByArtistAlbumController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let request_type = request.message.request_type;
        let mut bytes: BytesMut = request.to_response();
        let request_type_value = request_type.value();

        bytes.extend(Bytes::from(DBField::from([0x40, 0x00])));
        // TODO: Implement std::iter::Extend for BytesMut
        // bytes.extend(ArgumentCollection::new(vec![]));
        bytes.extend(Bytes::from(ArgumentCollection::new(vec![
            DBField::from([0x00, 0x00, request_type_value[0], request_type_value[1]]),
            DBField::from(2u32),
        ])));

        Bytes::from(bytes)
    }
}

struct MetadataController;
impl Controller for MetadataController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let request_type = request.message.request_type;
        let mut bytes: BytesMut = request.to_response();
        let request_type_value = request_type.value();

        bytes.extend(Bytes::from(DBField::from([0x40, 0x00])));
        bytes.extend(Bytes::from(ArgumentCollection::new(vec![
            DBField::from([0x00, 0x00, request_type_value[0], request_type_value[1]]),
            DBField::from(10u32),
        ])));

        Bytes::from(bytes)
    }
}

struct LoadTrackController;
impl Controller for LoadTrackController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let request_type = request.message.request_type;
        let mut bytes: BytesMut = request.to_response();
        let request_type_value = request_type.value();

        bytes.extend(Bytes::from(DBField::from([0x4e, 0x02])));
        bytes.extend(Bytes::from(
            ArgumentCollection::new(vec![
                DBField::from([0u8, 0u8, request_type_value[0], request_type_value[1]]),
                DBField::from(1u32),
                DBField::from(0u32),
                DBField::new(DBFieldType::Binary, &[]),
                DBField::from(0u32),
            ]),
        ));

        Bytes::from(bytes)
    }
}

impl Controller for RenderController {
    fn to_response(&self, request: RequestWrapper, context: &ClientState) -> Bytes {
        let response = Response::new();

        Bytes::from(match context.previous_request {
            Some(DBRequestType::RootMenuRequest) => self.render_root_menu(request, response, context),
            Some(DBRequestType::ArtistRequest) => self.render_artist_page(request, response, context),
            Some(DBRequestType::TitleRequest) => self.render_title_page(request, response, context),
            Some(DBRequestType::AlbumByArtistRequest) => self.render_album_by_artist(request, response, context),
            Some(DBRequestType::TitleByArtistAlbumRequest) => self.render_title_by_artist_album(request, response, context),
            Some(DBRequestType::MetadataRequest) => self.render_metadata(request, response, context),
            Some(DBRequestType::MountInfoRequest) => self.render_mount_info(request, response, context),
            _ => Response { buffer: BytesMut::new() },
        })
    }
}

fn get_controller(request_type: &DBRequestType) -> Option<Box<dyn Controller>> {
    match request_type {
        DBRequestType::AlbumByArtistRequest => Some(Box::new(NavigationController)),
        DBRequestType::ArtistRequest => Some(Box::new(NavigationController)),
        DBRequestType::LoadTrackRequest => Some(Box::new(LoadTrackController)),
        DBRequestType::MetadataRequest => Some(Box::new(MetadataController)),
        DBRequestType::MountInfoRequest => Some(Box::new(QueryMountInfoController)),
        DBRequestType::PreviewWaveformRequest => Some(Box::new(PreviewWaveformController)),
        DBRequestType::RenderRequest => Some(Box::new(RenderController)),
        DBRequestType::RootMenuRequest => Some(Box::new(RootMenuController)),
        DBRequestType::Setup => Some(Box::new(SetupController)),
        DBRequestType::TitleByArtistAlbumRequest => Some(Box::new(TitleByArtistAlbumController)),
        DBRequestType::TitleRequest => Some(Box::new(TitleController)),
        _ => None,
    }
}

fn process(
    bytes: Bytes,
    context: &mut ClientState,
    peer_addr: &SocketAddr,
) -> Bytes {
    if bytes.len() == 5 {
        return Bytes::from(bytes);
    }

    match DBMessage::parse(&bytes) {
        Ok((_unprocessed_bytes, message)) => {
            eprintln!("{:?}, {:?}", message.request_type, context.previous_request);
            //eprintln!("previous_request: {:?}\nrequest_type => {:?}\narguments => {:#?}\npeer: {:?}\n",
            //    context.previous_request,
            //    message.request_type,
            //    message.arguments,
            //    peer_addr);

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
                eprintln!("Not covered: {:?}", bytes);
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
                    process(Bytes::from(bytes), &mut context, &peer_addr)
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
                let port: u16 = rand::thread_rng().gen_range(60315, 65315);
                let state = state.clone();
                let allocated_port = port.to_be_bytes().to_vec();

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
    use super::super::fixtures;
    use pretty_assertions::{assert_eq};

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
    fn test_root_menu_dialog() {
        let dialog = fixtures::root_menu_dialog();
        let mut context = ClientState::new(SharedState::new());
        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1234);

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::RootMenuRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_artist_dialog_response() {
        let dialog = fixtures::artist_dialog();
        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1234);
        let mut context = ClientState::new(SharedState::new());

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::ArtistRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_album_by_artist_dialog() {
        let dialog = fixtures::album_by_artist_dialog();
        let mut context = ClientState::new(SharedState::new());
        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1234);

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::AlbumByArtistRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_title_by_artist_dialog() {
        let dialog = fixtures::title_by_artist_album_dialog();
        let mut context = ClientState::new(SharedState::new());
        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1234);

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::TitleByArtistAlbumRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_title_by_artist_dialog_single_track() {
        let dialog = fixtures::title_by_artist_album_single_track_dialog();
        let mut context = ClientState::new(SharedState::new());
        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1234);

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::TitleByArtistAlbumRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_metadata_dialog() {
        let dialog = fixtures::metadata_dialog();
        let mut context = ClientState::new(SharedState::new());
        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1234);

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::MetadataRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_mount_info_dialog() {
        let dialog = fixtures::mount_info_request_dialog();
        let mut context = ClientState::new(SharedState::new());
        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1234);

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::MountInfoRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_preview_waveform_request() {
        let dialog = fixtures::preview_waveform_request();
        let mut context = ClientState::new(SharedState::new());
        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1234);

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
    }

    #[test]
    fn test_load_track_request() {
        let dialog = fixtures::load_track_request();
        let mut context = ClientState::new(SharedState::new());
        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1234);

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::LoadTrackRequest), context.previous_request);
    }
}
