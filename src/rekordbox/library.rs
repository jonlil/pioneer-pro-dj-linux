use bytes::{Bytes, BytesMut};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_util::codec::{BytesCodec, Framed};

use super::db_field::{DBField, DBFieldType};
use super::db_message_argument::ArgumentCollection;
use super::db_request_type::DBRequestType;
use super::packets::{Arguments, DBMessage, ManyDBMessages};
use crate::rekordbox::{Database, Record, ServerState};
use crate::utils::network::random_ipv4_socket_address;
use futures::{SinkExt, StreamExt};

mod codec;
pub mod database;
mod fixtures;
mod helper;
pub mod metadata_type;
pub mod model;
mod request;

use fixtures::PREVIEW_WAVEFORM_RESPONSE;
use helper::*;
pub use metadata_type::*;
use request::{Controller, RequestHandler, RequestWrapper};

pub struct ClientState {
    previous_request: Option<StatefulRequest>,
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

    fn set_previous_request(&mut self, previous_request: StatefulRequest) {
        self.previous_request = Some(previous_request);
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
    fn to_response(&self, request: RequestWrapper, _context: &mut ClientState) -> Bytes {
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
    fn to_response(&self, request: RequestWrapper, _context: &mut ClientState) -> Bytes {
        let mut bytes: BytesMut = request.to_response();

        bytes.extend(ok_request());
        bytes.extend(Bytes::from(ArgumentCollection::new(vec![
            DBField::from([0x00, 0x00, 0x10, 0x00]),
            DBField::from([0x00, 0x00, 0x00, 0x08]),
        ])));

        Bytes::from(bytes)
    }
}

struct AlbumByArtistController;
impl Controller for AlbumByArtistController {
    fn to_response(&self, request: RequestWrapper, context: &mut ClientState) -> Bytes {
        let request_type = &request.message.request_type.value();
        let artist_id = dbfield_to_u32(&request.message.arguments[2]);

        let mut bytes: BytesMut = request.to_response();
        bytes.extend(ok_request());
        bytes.extend(Bytes::from(ArgumentCollection::new(vec![
            DBField::from([0u8, 0u8, request_type[0], request_type[1]]),
            DBField::from(number_of_tracks_by_artist(artist_id, &context.database)),
        ])));

        context.set_previous_request(StatefulRequest::AlbumByArtistRequest { artist_id });

        Bytes::from(bytes)
    }
}

struct ArtistController;
impl Controller for ArtistController {
    fn to_response(&self, request: RequestWrapper, context: &mut ClientState) -> Bytes {
        let request_type = &request.message.request_type.value();
        let mut bytes: BytesMut = request.to_response();

        bytes.extend(ok_request());
        bytes.extend(Bytes::from(ArgumentCollection::new(vec![
            DBField::from([0u8, 0u8, request_type[0], request_type[1]]),
            DBField::from(number_of_artists(&context.database)),
        ])));

        Bytes::from(bytes)
    }
}

struct PreviewWaveformController;
impl Controller for PreviewWaveformController {
    fn to_response(&self, request: RequestWrapper, _context: &mut ClientState) -> Bytes {
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
    fn to_response(&self, request: RequestWrapper, _context: &mut ClientState) -> Bytes {
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
        },
    )
}

fn build_message_footer(transaction_id: &DBField) -> DBMessage {
    DBMessage::new(
        transaction_id.clone(),
        DBRequestType::MenuFooter,
        ArgumentCollection::new(vec![DBField::from(1u32), DBField::from(1u32)]),
    )
}

struct RenderController;
impl RenderController {
    fn render_root_menu(&self, request: RequestWrapper, _context: &ClientState) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id.clone();
        let mut response = ManyDBMessages::new(vec![build_message_header(&transaction_id)]);

        response.extend(
            vec![
                // MenuName, MetadataType, MenuId
                ("\u{fffa}ARTIST\u{fffb}", metadata_type::ROOT_ARTIST, 0x02),
                ("\u{fffa}ALBUM\u{fffb}", metadata_type::ROOT_ALBUM, 0x03),
                ("\u{fffa}TRACK\u{fffb}", metadata_type::ROOT_TRACK, 0x04),
                ("\u{fffa}KEY\u{fffb}", metadata_type::ROOT_KEY, 0x0c),
                (
                    "\u{fffa}PLAYLIST\u{fffb}",
                    metadata_type::ROOT_PLAYLIST,
                    0x05,
                ),
                ("\u{fffa}HISTORY\u{fffb}", metadata_type::ROOT_HISTORY, 0x16),
                ("\u{fffa}SEARCH\u{fffb}", metadata_type::ROOT_SEARCH, 0x12),
            ]
            .iter()
            .map(|item| build_message_item(&transaction_id, item.0, item.1, item.2))
            .collect(),
        );
        response.push(build_message_footer(&transaction_id));

        response
    }

    fn render_artist_page(&self, request: RequestWrapper, context: &ClientState) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id;
        let mut response = ManyDBMessages::new(vec![build_message_header(&transaction_id)]);

        for artist in context.database.artists() {
            response.push(build_message_item(
                &transaction_id,
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

        let mut response = ManyDBMessages::new(vec![build_message_header(&transaction_id)]);
        response.push(build_message_item(
            &transaction_id,
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

    fn render_album_by_artist(
        &self,
        request: RequestWrapper,
        _context: &ClientState,
        _artist_id: u32,
    ) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id;
        let mut response = ManyDBMessages::new(vec![build_message_header(&transaction_id)]);

        response.push(build_message_item(
            &transaction_id,
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

    fn render_title_by_artist_album(
        &self,
        request: RequestWrapper,
        context: &ClientState,
        artist_id: u32,
    ) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id;
        let tracks = context.database.title_by_artist(artist_id);

        let mut response = ManyDBMessages::new(vec![build_message_header(&transaction_id)]);

        for track in tracks {
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

    fn render_metadata(
        &self,
        request: RequestWrapper,
        context: &ClientState,
        track_id: u32,
    ) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id;
        let track = context.database.get_track(track_id).unwrap();
        let artist = context.database.get_artist(track.artist_id).unwrap();

        ManyDBMessages::new(vec![
            build_message_header(&transaction_id),
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                Arguments {
                    entry_id1: 1,
                    entry_id2: 5,
                    entry_id4: 256,
                    value1: track.name(),
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
                    value1: artist.name(),
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
                    entry_id2: track.bpm.unwrap_or(0),
                    _type: metadata_type::BPM,
                    ..Default::default()
                },
            ),
            DBMessage::new(
                transaction_id.clone(),
                DBRequestType::MenuItem,
                Arguments {
                    entry_id2: 5,
                    value1: "",
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

    fn render_mount_info(
        &self,
        request: RequestWrapper,
        context: &ClientState,
        track_id: u32,
    ) -> ManyDBMessages {
        let transaction_id = request.message.transaction_id;

        let mut resp = ManyDBMessages::new(vec![build_message_header(&transaction_id)]);

        match context.database.get_track(track_id) {
            Some(track) => {
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
                        entry_id2: track.bpm.unwrap_or(0),
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
                        entry_id1: track.size,
                        entry_id2: 5,
                        value1: track.path(),
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
            }
            None => panic!("Should not happen"),
        };

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
    fn to_response(&self, request: RequestWrapper, context: &mut ClientState) -> Bytes {
        let request_type_value = request.message.request_type.value();
        let items_to_render: u32 = 6u32;
        let track_id = dbfield_to_u32(&request.message.arguments[1]);

        context.set_previous_request(StatefulRequest::MountInfoRequest { track_id });

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
    fn to_response(&self, request: RequestWrapper, context: &mut ClientState) -> Bytes {
        let artist_id = dbfield_to_u32(&request.message.arguments[2]);
        let request_type_value = request.message.request_type.value();
        let number_of_tracks_by_artist = number_of_tracks_by_artist(artist_id, &context.database);

        context.set_previous_request(StatefulRequest::TitleByArtistAlbumRequest { artist_id });

        Bytes::from(DBMessage::new(
            request.message.transaction_id,
            DBRequestType::Success,
            ArgumentCollection::new(vec![
                DBField::from([0x00, 0x00, request_type_value[0], request_type_value[1]]),
                DBField::from(number_of_tracks_by_artist),
            ]),
        ))
    }
}

struct MetadataController;
impl Controller for MetadataController {
    fn to_response(&self, request: RequestWrapper, context: &mut ClientState) -> Bytes {
        let request_type_value = request.message.request_type.value();
        let track_id = dbfield_to_u32(&request.message.arguments[1]);

        context.set_previous_request(StatefulRequest::MetadataRequest { track_id });

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
    fn to_response(&self, request: RequestWrapper, _context: &mut ClientState) -> Bytes {
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

#[derive(Debug, PartialEq)]
enum StatefulRequest {
    RootMenuRequest,
    ArtistRequest,
    TitleRequest,
    AlbumByArtistRequest { artist_id: u32 },
    TitleByArtistAlbumRequest { artist_id: u32 },
    MetadataRequest { track_id: u32 },
    MountInfoRequest { track_id: u32 },
}

impl Controller for RenderController {
    fn to_response(&self, request: RequestWrapper, context: &mut ClientState) -> Bytes {
        Bytes::from(match context.previous_request {
            Some(StatefulRequest::RootMenuRequest) => self.render_root_menu(request, context),
            Some(StatefulRequest::ArtistRequest) => self.render_artist_page(request, context),
            Some(StatefulRequest::TitleRequest) => self.render_title_page(request, context),
            Some(StatefulRequest::AlbumByArtistRequest { artist_id }) => {
                self.render_album_by_artist(request, context, artist_id)
            }
            Some(StatefulRequest::TitleByArtistAlbumRequest { artist_id }) => {
                self.render_title_by_artist_album(request, context, artist_id)
            }
            Some(StatefulRequest::MetadataRequest { track_id }) => {
                self.render_metadata(request, context, track_id)
            }
            Some(StatefulRequest::MountInfoRequest { track_id }) => {
                self.render_mount_info(request, context, track_id)
            }
            _ => ManyDBMessages::new(vec![]),
        })
    }
}

fn get_controller(request_type: &DBRequestType) -> Option<Box<dyn Controller>> {
    match request_type {
        DBRequestType::AlbumByArtistRequest => Some(Box::new(AlbumByArtistController)),
        DBRequestType::ArtistRequest => Some(Box::new(ArtistController)),
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

/// This is a post processor of the request
///
/// Some Controllers will extract some data from the request that is required
/// for executing a future client request.
fn handle_sequence_requests(context: &mut ClientState, request_type: &DBRequestType) {
    match request_type {
        DBRequestType::AlbumByArtistRequest => {}
        DBRequestType::TitleByArtistAlbumRequest => {}
        DBRequestType::ArtistRequest => {
            context.set_previous_request(StatefulRequest::ArtistRequest)
        }
        DBRequestType::TitleRequest => context.set_previous_request(StatefulRequest::TitleRequest),
        DBRequestType::RootMenuRequest => {
            context.set_previous_request(StatefulRequest::RootMenuRequest)
        }
        DBRequestType::MetadataRequest => {}
        DBRequestType::MountInfoRequest => {}
        _ => {}
    };
}

fn process(bytes: Bytes, context: &mut ClientState, _peer: &SocketAddr) -> Bytes {
    // TODO: Before implementing DbBytesCodec this must be migrated.
    if bytes.len() == 5 {
        return Bytes::from(bytes);
    }

    match DBMessage::parse(&bytes) {
        Ok((_unprocessed_bytes, message)) => {
            if let Some(request_handler) = get_controller(&message.request_type) {
                handle_sequence_requests(context, &message.request_type);

                return RequestHandler::new(request_handler, message, context).respond_to();
            } else {
                eprintln!(
                    "DBRequestType: {:?} has no controller implemented.\nRaw bytes: {:?}",
                    &message.request_type, bytes
                );

                return DBMessage::new(
                    message.transaction_id,
                    DBRequestType::Success,
                    ArgumentCollection::new(vec![]),
                )
                .into();
            }
        }
        Err(nom::Err::Error(bytes)) => eprintln!("Error: {:?}", bytes),
        _ => eprintln!("Not covered: {:?}", bytes),
    }

    Bytes::from("panic")
}

async fn spawn_library_client_handler(
    mut listener: TcpListener,
    state: &Arc<Mutex<ServerState>>,
    database: &Arc<Database>,
) {
    match listener.accept().await {
        Ok((remote_client, address)) => {
            let mut remote_client = Framed::new(remote_client, BytesCodec::new());
            let mut context = ClientState::new(state.clone(), database.clone());

            while let Some(result) = remote_client.next().await {
                match result {
                    Ok(data) => {
                        match remote_client
                            .send(process(data.freeze(), &mut context, &address))
                            .await
                        {
                            Ok(_) => {}
                            Err(err) => {
                                eprintln!("failed sending library query response; error = {}", err)
                            }
                        }
                    }
                    Err(err) => eprintln!("library client handler got error; error = {}", err),
                }
            }
        }
        Err(err) => eprintln!("failed reading connection on socket; error = {}", err),
    }
}

pub struct DBLibraryServer;
impl DBLibraryServer {
    async fn spawn(
        address: &str,
        state: Arc<Mutex<ServerState>>,
        database: Arc<Database>,
    ) -> Result<(), std::io::Error> {
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
                                    let allocated_socket =
                                        TcpListener::bind(&random_ipv4_socket_address())
                                            .await
                                            .unwrap();
                                    let allocated_port =
                                        allocated_socket.local_addr().unwrap().port();

                                    tokio::spawn(async move {
                                        spawn_library_client_handler(
                                            allocated_socket,
                                            &state,
                                            &database,
                                        )
                                        .await;
                                    });
                                    let message =
                                        Bytes::from(allocated_port.to_be_bytes().to_vec());
                                    match socket.send(message).await {
                                        Err(err) => eprintln!("failed sending library server port to client; error = {}", err),
                                        _ => {},
                                    }
                                }
                                Err(_err) => {}
                            };
                        }
                    });
                }
                Err(err) => eprintln!("error accepting socket: {}", err),
            }
        }
    }

    pub async fn run(
        state: Arc<Mutex<ServerState>>,
        database: Arc<Database>,
    ) -> Result<(), std::io::Error> {
        Self::spawn("0.0.0.0:12523", state, database).await
    }
}

#[cfg(test)]
mod test {
    use super::super::fixtures;
    use super::*;
    use crate::rekordbox::{Database, ServerState};
    use pretty_assertions::assert_eq;
    use std::net::{IpAddr, Ipv4Addr};

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
        fn to_response(&self, _request: RequestWrapper, _context: &mut ClientState) -> Bytes {
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

        assert_eq!(
            request_handler.respond_to(),
            Bytes::from("my-very-test-value")
        );
    }

    #[test]
    fn test_setup_request_handling() {
        let mut context = context();
        let request_handler = RequestHandler::new(
            Box::new(SetupController {}),
            fixtures::setup_request_packet().unwrap().1,
            &mut context,
        );

        assert_eq!(
            request_handler.respond_to(),
            fixtures::setup_response_packet()
        );
    }

    #[test]
    fn test_album_by_artist_dialog() {
        let dialog = fixtures::album_by_artist_dialog();
        let mut context = context();
        let peer_addr = peer();

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(
            Some(StatefulRequest::AlbumByArtistRequest { artist_id: 0u32 }),
            context.previous_request,
        );
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_title_by_artist_dialog() {
        let dialog = fixtures::title_by_artist_album_dialog();
        let mut context = context();
        let peer_addr = peer();

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(
            Some(StatefulRequest::TitleByArtistAlbumRequest { artist_id: 0u32 }),
            context.previous_request,
        );
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }

    #[test]
    fn test_title_by_artist_dialog_single_track() {
        let dialog = fixtures::title_by_artist_album_single_track_dialog();
        let mut context = context();
        let peer_addr = peer();

        assert_eq!(dialog.1, process(dialog.0, &mut context, &peer_addr));
        assert_eq!(
            Some(StatefulRequest::TitleByArtistAlbumRequest { artist_id: 0u32 }),
            context.previous_request
        );
        assert_eq!(dialog.3, process(dialog.2, &mut context, &peer_addr));
    }
}
