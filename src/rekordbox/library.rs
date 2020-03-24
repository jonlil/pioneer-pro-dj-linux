use bytes::{Bytes, BytesMut};
use std::net::{SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::stream::StreamExt;
use tokio_util::codec::{Framed, BytesCodec};
use futures::SinkExt;

use super::packets::{DBMessage, ManyDBMessages, Arguments};
use super::db_field::{DBField, DBFieldType};
use super::db_request_type::DBRequestType;
use super::db_message_argument::ArgumentCollection;
use crate::rekordbox::{Database, ServerState, Record};
use crate::utils::network::random_ipv4_socket_address;

mod codec;
mod request;
mod fixtures;
mod helper;
pub mod model;
pub mod database;
pub mod metadata_type;

pub use metadata_type::*;
use request::{Controller, RequestWrapper, RequestHandler};
use fixtures::PREVIEW_WAVEFORM_RESPONSE;
use helper::*;

pub struct ClientState {
    previous_request: Option<DBRequestType>,
    state: Arc<Mutex<ServerState>>,
    database: Arc<Database>,
}

impl ClientState {
    pub fn new(state: Arc<Mutex<ServerState>>, database: Arc<Database>) -> Self {
        Self {
            previous_request: None,
            state,
            database,
        }
    }
}

pub enum Event {
    RemoteDBServer,
    Unsupported,
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
    fn to_response(&self, request: RequestWrapper, context: &ClientState) -> Bytes {
        let request_type = request.message.request_type;
        let items_to_render = match request_type {
            DBRequestType::ArtistRequest => number_of_artists(&context.database),
            DBRequestType::AlbumByArtistRequest => {
                let artist_id = dbfield_to_u32(&request.message.arguments[2]);
                number_of_tracks_by_artist(artist_id, &context.database)
            },
            _ => {
                dbg!(request_type);
                0u32
            },
        };

        let mut bytes: BytesMut = request.to_response();
        let request_type_value = request_type.value();

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
            DBField::new(DBFieldType::Binary, &PREVIEW_WAVEFORM_RESPONSE),
        ])));

        Bytes::from(bytes)
    }
}

struct TitleController;
impl Controller for TitleController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        request.to_response().freeze()
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

fn build_message_item(
    transaction_id: &DBField,
    value1: &str,
    entry_type: u32,
    entry_id2: u32,
) -> DBMessage {
    DBMessage::new(
        transaction_id.clone(),
        DBRequestType::MenuItem,
        Arguments {
            entry_id2,
            value1: value1,
            _type: entry_type,
            ..Default::default()
        }
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
    fn render_root_menu(&self, request: RequestWrapper, _context: &ClientState) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id.clone();
        let mut response = ManyDBMessages::new(vec![
            build_message_header(&transaction_id),
        ]);

        response.extend(vec![
            // MenuName, MetadataType, MenuId
            ("\u{fffa}ARTIST\u{fffb}", metadata_type::ROOT_ARTIST,      0x02),
            ("\u{fffa}ALBUM\u{fffb}", metadata_type::ROOT_ALBUM,        0x03),
            ("\u{fffa}TRACK\u{fffb}", metadata_type::ROOT_TRACK,        0x04),
            ("\u{fffa}KEY\u{fffb}", metadata_type::ROOT_KEY,            0x0c),
            ("\u{fffa}PLAYLIST\u{fffb}", metadata_type::ROOT_PLAYLIST,  0x05),
            ("\u{fffa}HISTORY\u{fffb}", metadata_type::ROOT_HISTORY,    0x16),
            ("\u{fffa}SEARCH\u{fffb}", metadata_type::ROOT_SEARCH,      0x12),
        ].iter().map(|item| build_message_item(&transaction_id,
            item.0,
            item.1,
            item.2,
        )).collect());
        response.push(build_message_footer(&transaction_id));

        response
    }

    fn render_artist_page(&self, request: RequestWrapper, context: &ClientState) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id;
        let mut response = ManyDBMessages::new(vec![
            build_message_header(&transaction_id),
        ]);

        for artist in context.database.artists() {
            response.push(build_message_item(&transaction_id,
                artist.name().as_str(),
                metadata_type::ARTIST,
                *artist.id(),
            ));
        }

        response.push(DBMessage::new(
            transaction_id,
            DBRequestType::MenuFooter,
            ArgumentCollection::new(vec![]),
        ));

        response
    }

    fn render_title_page(&self, request: RequestWrapper, _context: &ClientState) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id;

        let mut response = ManyDBMessages::new(vec![
            build_message_header(&transaction_id),
        ]);
        response.push(build_message_item(&transaction_id,
            "Loopmasters",
            metadata_type::TITLE,
            0x05,
        ));
        response.push(DBMessage::new(
            transaction_id,
            DBRequestType::MenuFooter,
            ArgumentCollection::new(vec![]),
        ));

        response
    }

    fn render_album_by_artist(&self, request: RequestWrapper, _context: &ClientState) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id;
        let mut response = ManyDBMessages::new(vec![
            build_message_header(&transaction_id),
        ]);

        response.push(build_message_item(&transaction_id,
            "Unknown",
            metadata_type::ALBUM,
            0x00,
        ));

        response.push(DBMessage::new(
            transaction_id,
            DBRequestType::MenuFooter,
            ArgumentCollection::new(vec![]),
        ));

        response
    }

    fn render_title_by_artist_album(&self, request: RequestWrapper, context: &ClientState) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id;
        let artist_id = dbfield_to_u32(&request.message.arguments[2]);

        let mut response = ManyDBMessages::new(vec![
            build_message_header(&transaction_id),
        ]);
        for track in context.database.title_by_artist(artist_id) {
            response.push(build_message_item(
                &transaction_id,
                &track.name().clone(),
                metadata_type::TITLE,
                *track.id(),
            ));
        }

        response.push(DBMessage::new(
            transaction_id,
            DBRequestType::MenuFooter,
            ArgumentCollection::new(vec![]),
        ));

        response
    }

    fn render_metadata(&self, request: RequestWrapper, _context: &ClientState) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id;

        ManyDBMessages::new(vec![
            build_message_header(&transaction_id),
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                Arguments {
                    entry_id1: 1,
                    entry_id2: 5,
                    entry_id4: 256,
                    value1: "Demo Track 1",
                    _type: metadata_type::TITLE,
                    ..Default::default()
                },
            ),
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                Arguments {
                    entry_id1: 1,
                    entry_id2: 1,
                    value1: "Loopmasters",
                    _type: metadata_type::ARTIST,
                    ..Default::default()
                },
            ),
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                Arguments {
                    entry_id1: 1,
                    _type: metadata_type::ALBUM,
                    ..Default::default()
                },
            ),
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                Arguments {
                    entry_id2: 195,
                    _type: metadata_type::DURATION,
                    ..Default::default()
                },
            ),
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                Arguments {
                    entry_id2: 12800,
                    _type: metadata_type::BPM,
                    ..Default::default()
                },
            ),
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                Arguments {
                    entry_id2: 5,
                    value1: "Tracks by www.loopmasters.com",
                    _type: metadata_type::COMMENT,
                    ..Default::default()
                },
            ),
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                Arguments {
                    entry_id1: 1,
                    _type: metadata_type::KEY,
                    ..Default::default()
                },
            ),
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                Arguments {
                    _type: metadata_type::RATING,
                    ..Default::default()
                },
            ),
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                Arguments {
                    _type: metadata_type::COLOR_NONE,
                    ..Default::default()
                },
            ),
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                Arguments {
                    _type: metadata_type::GENRE,
                    ..Default::default()
                },
            ),
            DBMessage::new(
                transaction_id,
                DBRequestType::MenuFooter,
                ArgumentCollection::new(vec![]),
            ),
        ])
    }

    fn render_mount_info(&self, request: RequestWrapper, _context: &ClientState) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id;

        let mut resp = ManyDBMessages::new(vec![
            build_message_header(&transaction_id),
        ]);

        resp.push(DBMessage::new(
            transaction_id.clone(),
            DBRequestType::MenuItem,
            Arguments {
                _type: metadata_type::TITLE,
                entry_id2: 1,
                ..Default::default()
            },
        ));

        resp.push(DBMessage::new(
            transaction_id.clone(),
            DBRequestType::MenuItem,
            Arguments {
                _type: metadata_type::DURATION,
                entry_id2: 195,
                ..Default::default()
            },
        ));
        resp.push(DBMessage::new(
            transaction_id.clone(),
            DBRequestType::MenuItem,
            Arguments {
                _type: metadata_type::BPM,
                entry_id2: 12800,
                ..Default::default()
            },
        ));
        resp.push(DBMessage::new(
            transaction_id.clone(),
            DBRequestType::MenuItem,
            Arguments {
                _type: metadata_type::COMMENT,
                value1: "Tracks by www.loopmasters.com",
                ..Default::default()
            },
        ));
        resp.push(DBMessage::new(
            transaction_id.clone(),
            DBRequestType::MenuItem,
            Arguments {
                _type: metadata_type::MOUNT_PATH,
                entry_id1: 7869988,
                entry_id2: 5,
                value1: "/home/jonas/Music/PioneerDJ/Demo Tracks/Demo Track 1.mp3",
                ..Default::default()
            },
        ));
        resp.push(DBMessage::new(
            transaction_id.clone(),
            DBRequestType::MenuItem,
            Arguments {
                _type: metadata_type::UNKNOWN1,
                entry_id2: 1,
                ..Default::default()
            },
        ));

        resp.push(DBMessage::new(
            transaction_id,
            DBRequestType::MenuFooter,
            ArgumentCollection::new(vec![]),
        ));

        resp
    }
}

struct QueryMountInfoController;
impl Controller for QueryMountInfoController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let request_type_value = request.message.request_type.value();
        let items_to_render: u32 = 6u32;

        Bytes::from(DBMessage::new(
            request.message.transaction_id,
            DBRequestType::Success,
            ArgumentCollection::new(vec![
                DBField::from([0u8, 0u8, request_type_value[0], request_type_value[1]]),
                DBField::from(items_to_render),
            ]),
        ))
    }
}

fn dbfield_to_u32(input: &DBField) -> u32 {
    if input.kind != DBFieldType::U32 {
        panic!("Unsupported conversation");
    }

    let mut inner_value: [u8; 4] = [0u8; 4];
    let mut index = 0;
    for val in input.value[..=3].iter() {
        inner_value[index] = *val;
        index += 1;
    }
    u32::from_be_bytes(inner_value)
}

struct TitleByArtistAlbumController;
impl Controller for TitleByArtistAlbumController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let request_type_value = request.message.request_type.value();
        let number_of_tracks_by_artist = 5u32;

        Bytes::from(DBMessage::new(
            request.message.transaction_id,
            DBRequestType::Success,
            ArgumentCollection::new(vec![
                DBField::from([0x00, 0x00, request_type_value[0], request_type_value[1]]),
                DBField::from(number_of_tracks_by_artist),
            ])
        ))
    }
}

struct MetadataController;
impl Controller for MetadataController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let request_type_value = request.message.request_type.value();

        Bytes::from(DBMessage::new(
            request.message.transaction_id,
            DBRequestType::Success,
            ArgumentCollection::new(vec![
                DBField::from([0x00, 0x00, request_type_value[0], request_type_value[1]]),
                DBField::from(10u32),
            ]),
        ))
    }
}

struct LoadTrackController;
impl Controller for LoadTrackController {
    fn to_response(&self, request: RequestWrapper, _context: &ClientState) -> Bytes {
        let request_type_value = request.message.request_type.value();

        Bytes::from(DBMessage::new(
            request.message.transaction_id,
            DBRequestType::LoadTrackSuccess,
            ArgumentCollection::new(vec![
                DBField::from([0u8, 0u8, request_type_value[0], request_type_value[1]]),
                DBField::from(1u32),
                DBField::from(0u32),
                DBField::new(DBFieldType::Binary, &[]),
                DBField::from(0u32),
            ]),
        ))
    }
}

impl Controller for RenderController {
    fn to_response(&self, request: RequestWrapper, context: &ClientState) -> Bytes {
        Bytes::from(match context.previous_request {
            Some(DBRequestType::RootMenuRequest) => self.render_root_menu(request, context),
            Some(DBRequestType::ArtistRequest) => self.render_artist_page(request, context),
            Some(DBRequestType::TitleRequest) => self.render_title_page(request, context),
            Some(DBRequestType::AlbumByArtistRequest) => self.render_album_by_artist(request, context),
            Some(DBRequestType::TitleByArtistAlbumRequest) => self.render_title_by_artist_album(request, context),
            Some(DBRequestType::MetadataRequest) => self.render_metadata(request, context),
            Some(DBRequestType::MountInfoRequest) => self.render_mount_info(request, context),
            _ => ManyDBMessages::new(vec![]),
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
    _peer: &SocketAddr,
) -> Bytes {
    // TODO: Before implementing DbBytesCodec this must be migrated.
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
                eprintln!("Not covered: {:?}", bytes);
            }
        },
        Err(nom::Err::Error((bytes, _))) => eprintln!("Error: {:?}", bytes),
        _ => eprintln!("Not covered: {:?}", bytes),
    }

    Bytes::from("Failed processing request into response")
}

async fn spawn_library_client_handler(
    mut listener: TcpListener,
    state: &Arc<Mutex<ServerState>>,
    database: &Arc<Database>
) {
    match listener.accept().await {
        Ok((remote_client, address)) => {
            let mut remote_client = Framed::new(remote_client, BytesCodec::new());
            let mut context = ClientState::new(state.clone(), database.clone());

            while let Some(result) = remote_client.next().await {
                match result {
                    Ok(data) => {
                        match remote_client.send(process(data.freeze(), &mut context, &address)).await {
                            Ok(_) => {},
                            Err(err) => eprintln!("failed sending library query response; error = {}", err),
                        }
                    },
                    Err(err) => eprintln!("library client handler got error; error = {}", err),
                }
            }
        },
        Err(err) => eprintln!("failed reading connection on socket; error = {}", err),
    }
}

pub struct DBLibraryServer;
impl DBLibraryServer {
    async fn spawn(address: &str, state: Arc<Mutex<ServerState>>, database: Arc<Database>) -> Result<(), std::io::Error> {
        let addr = address.parse::<SocketAddr>().unwrap();
        let mut listener = TcpListener::bind(&addr).await?;

        loop {
            match listener.accept().await {
                Ok((socket, _address)) => {
                    let state = state.clone();
                    let database = database.clone();

                    tokio::spawn(async move {
                        let mut socket = Framed::new(socket, BytesCodec::new());

                        while let Some(result) = socket.next().await {
                            match result {
                                Ok(_data) => {
                                    let state = state.clone();
                                    let database = database.clone();
                                    let allocated_socket = TcpListener::bind(&random_ipv4_socket_address()).await.unwrap();
                                    let allocated_port = allocated_socket.local_addr().unwrap().port();

                                    tokio::spawn(async move {
                                        spawn_library_client_handler(allocated_socket, &state, &database).await;
                                    });
                                    let message = Bytes::from(allocated_port.to_be_bytes().to_vec());
                                    match socket.send(message).await {
                                        Err(err) => eprintln!("failed sending library server port to client; error = {}", err),
                                        _ => {},
                                    }
                                },
                                Err(_err) => {},
                            };
                        }
                    });
                },
                Err(err) => eprintln!("error accepting socket: {}", err),
            }
        }
    }

    pub async fn run(state: Arc<Mutex<ServerState>>, database: Arc<Database>) -> Result<(), std::io::Error> {
        Self::spawn("0.0.0.0:12523", state, database).await
    }
}

#[cfg(test)]
mod test {
    use std::net::{Ipv4Addr, IpAddr};
    use super::*;
    use super::super::fixtures;
    use pretty_assertions::{assert_eq};
    use crate::rekordbox::{ServerState, Database};

    fn context() -> ClientState {
        ClientState::new(
            Arc::new(Mutex::new(ServerState::new())),
            Arc::new(Database::new("./test/music")),
        )
    }

    fn peer() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1234)
    }

    pub struct TestController;
    impl Controller for TestController {
        fn to_response(&self, _request: RequestWrapper, _context: &ClientState) -> Bytes {
            Bytes::from("my-very-test-value")
        }
    }

    #[test]
    fn test_controller_trait() {
        let mut context = context();
        let request_handler = RequestHandler::new(
            Box::new(TestController {}),
            fixtures::setup_request_packet().unwrap().1,
            &mut context,
        );

        assert_eq!(request_handler.respond_to(), Bytes::from("my-very-test-value"));
    }

    #[test]
    fn test_setup_request_handling() {
        let mut context = context();
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
        let mut context = context();
        let peer_addr = peer();

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::RootMenuRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_artist_dialog_response() {
        let dialog = fixtures::artist_dialog();
        let peer_addr = peer();
        let mut context = context();

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::ArtistRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_album_by_artist_dialog() {
        let dialog = fixtures::album_by_artist_dialog();
        let mut context = context();
        let peer_addr = peer();

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::AlbumByArtistRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_title_by_artist_dialog() {
        let dialog = fixtures::title_by_artist_album_dialog();
        let mut context = context();
        let peer_addr = peer();

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::TitleByArtistAlbumRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_title_by_artist_dialog_single_track() {
        let dialog = fixtures::title_by_artist_album_single_track_dialog();
        let mut context = context();
        let peer_addr = peer();

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::TitleByArtistAlbumRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    #[ignore]
    fn test_metadata_dialog() {
        let dialog = fixtures::metadata_dialog();
        let mut context = context();
        let peer_addr = peer();

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::MetadataRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    #[ignore = "matches against hardcoded, should be enabled when we have a database."]
    fn test_mount_info_dialog() {
        let dialog = fixtures::mount_info_request_dialog();
        let mut context = context();
        let peer_addr = peer();

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::MountInfoRequest), context.previous_request);
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_preview_waveform_request() {
        let dialog = fixtures::preview_waveform_request();
        let mut context = context();
        let peer_addr = peer();

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
    }

    #[test]
    fn test_load_track_request() {
        let dialog = fixtures::load_track_request();
        let mut context = context();
        let peer_addr = peer();

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(Some(DBRequestType::LoadTrackRequest), context.previous_request);
    }
}
