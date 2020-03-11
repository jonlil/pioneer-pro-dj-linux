#![allow(dead_code)]
#![allow(non_snake_case)]

pub mod server;
pub mod packets;
mod codec;
mod fs;
mod nfs_program;

pub mod events {
    use super::packets::{
        RpcProcedure,
        RpcReplyMessage,
        RpcCall,
    };
    pub type RpcResult = Option<Result<RpcReplyMessage, std::io::Error>>;
    pub trait EventHandler: Send + Sync + 'static {
        fn on_event(
            &self,
            procedure: &RpcProcedure,
            call: &RpcCall
        ) -> RpcResult;

        fn handle_event(&self, call: &RpcCall) -> RpcResult {
            self.on_event(call.procedure(), call)
        }
    }
}

pub use server::PortmapServer;
