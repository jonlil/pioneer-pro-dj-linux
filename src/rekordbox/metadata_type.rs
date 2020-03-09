pub type MetadataType = u32;

pub const MOUNT_PATH: MetadataType = 0x00000000;
pub const FOLDER: MetadataType = 0x00000001;
pub const ALBUM: MetadataType = 0x00000002;
pub const TITLE: MetadataType = 0x00000004;
pub const GENRE: MetadataType = 0x00000006;
pub const ARTIST: MetadataType = 0x00000007;
pub const PLAYLIST: MetadataType = 0x00000008;
pub const RATING: MetadataType = 0x0000000a;
pub const DURATION: MetadataType = 0x0000000b;
pub const BPM: MetadataType = 0x0000000d;
pub const LABEL: MetadataType = 0x0000000e;
pub const KEY: MetadataType = 0x0000000f;
pub const COLOR_NONE: MetadataType = 0x00000013;
pub const UNKNOWN1: MetadataType = 0x0000002f;

pub const COMMENT: MetadataType = 0x00000023;
pub const ROOT_ARTIST: MetadataType = 0x00000081;
pub const ROOT_ALBUM: MetadataType = 0x00000082;
pub const ROOT_TRACK: MetadataType = 0x00000083;
pub const ROOT_PLAYLIST: MetadataType = 0x00000084;
pub const ROOT_RATING: MetadataType = 0x00000086;
pub const ROOT_KEY: MetadataType = 0x0000008b;
pub const ROOT_FOLDER: MetadataType = 0x00000090;
pub const ROOT_SEARCH: MetadataType = 0x00000091;
pub const ROOT_HISTORY: MetadataType = 0x00000095;

pub enum ArgumentType {
    MountPath,
    Folder,
    Album,
    Title,
    Genre,
    Artist,
    Playlist,
    Rating,
    Duration,
    Bpm,
    Label,
    Key,
    ColorNone,
    Unknown1,
    Comment,
    RootArtist,
    RootAlbum,
    RootTrack,
    RootPlaylist,
    RootRating,
    RootKey,
    RootFolder,
    RootSearch,
    RootHistory,
}
