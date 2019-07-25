use bytes::Bytes;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum DBRequestType {
    AlbumByArtistRequest,
    AlbumRequest,
    ArtistRequest,
    GenreRequest,
    HistoryRequest,
    KeyRequest,
    MenuFooter,
    MenuHeader,
    MenuItem,
    MetadataRequest,
    MountInfoRequest,
    LoadTrackRequest,
    LoadTrackSuccess,
    PlaylistRequest,
    PreviewWaveformRequest,
    RenderRequest,
    RootMenuRequest,
    SearchQueryRequest,
    Setup,
    Success,
    TitleByArtistAlbumRequest,
    TitleRequest,
    Unknown(u16),
}

impl DBRequestType {
    pub fn value(&self) -> Bytes {
        Bytes::from(match self {
            DBRequestType::AlbumByArtistRequest => "\x11\x02",
            DBRequestType::ArtistRequest => "\x10\x02",
            DBRequestType::LoadTrackRequest => "\x2b\x04",
            DBRequestType::MenuFooter => "\x42\x01",
            DBRequestType::MenuHeader => "\x40\x01",
            DBRequestType::MenuItem => "\x41\x01",
            DBRequestType::LoadTrackSuccess => "\x4e\x02",
            DBRequestType::MetadataRequest => "\x20\x02",
            DBRequestType::MountInfoRequest => "\x21\x02",
            DBRequestType::PreviewWaveformRequest => "\x20\x04",
            DBRequestType::RootMenuRequest => "\x10\x00",
            DBRequestType::RenderRequest => "\x30\x00",
            DBRequestType::Setup => "\x00\x00",
            DBRequestType::Success => "\x40\x00",
            DBRequestType::TitleByArtistAlbumRequest => "\x12\x02",
            _ => "\x00\x00",
        })
    }

    pub fn new(value: u16) -> DBRequestType {
        match value {
            0_u16    => DBRequestType::Setup,
            4096_u16 => DBRequestType::RootMenuRequest,
            4097_u16 => DBRequestType::GenreRequest,
            4098_u16 => DBRequestType::ArtistRequest,
            4099_u16 => DBRequestType::AlbumRequest,
            4100_u16 => DBRequestType::TitleRequest,
            4114_u16 => DBRequestType::HistoryRequest,
            4116_u16 => DBRequestType::KeyRequest,
            4354_u16 => DBRequestType::AlbumByArtistRequest,
            4357_u16 => DBRequestType::PlaylistRequest,
            4610_u16 => DBRequestType::TitleByArtistAlbumRequest,
            4864_u16 => DBRequestType::SearchQueryRequest,
            8194_u16 => DBRequestType::MetadataRequest,
            8196_u16 => DBRequestType::PreviewWaveformRequest,
            8450_u16 => DBRequestType::MountInfoRequest,
            11012_u16 => DBRequestType::LoadTrackRequest,
            12288_u16 => DBRequestType::RenderRequest,
            16384_u16 => DBRequestType::Success,
            16385_u16 => DBRequestType::MenuHeader,
            16641_u16 => DBRequestType::MenuItem,
            16897_u16 => DBRequestType::MenuFooter,
            _ => DBRequestType::Unknown(value)
        }
    }
}
