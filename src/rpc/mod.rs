#![allow(dead_code)]
#![allow(non_snake_case)]

extern crate tokio;
extern crate futures;
extern crate bytes;

pub mod server;
pub mod packets;
mod codec;

pub mod events {
    use super::packets::{
        RpcProcedure,
        RpcReplyMessage,
        RpcCall,
    };

    pub trait EventHandler: Send + Sync + 'static {
        fn on_event(&self, procedure: &RpcProcedure, call: &RpcCall) -> Result<RpcReplyMessage, std::io::Error>;

        fn handle_event(&self, call: &RpcCall) -> Result<RpcReplyMessage, std::io::Error> {
            self.on_event(call.procedure(), call)
        }
    }
}

pub use server::PortmapServer;
