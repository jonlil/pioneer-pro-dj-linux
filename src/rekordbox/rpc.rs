use std::collections::HashMap;
use crate::rpc::{RPCCall, RPCReply, RPC, Mount, self};

type Callback = fn(RPC) -> Option<RPCReply>;

pub struct EventHandler {
    callbacks: HashMap<String, Callback>,
    // mpsc
}

impl EventHandler {
    pub fn new() -> Self {
        let mut handler = EventHandler {
            callbacks: HashMap::new(),
        };

        handler.add_callback("Export".to_string(), rpc_method_export);
        handler.add_callback("MNT".to_string(), rpc_method_mnt);

        handler
    }

    fn add_callback(&mut self, name: String, func: Callback) {
        self.callbacks.insert(name, func);
    }
}

fn rpc_method_export(rpc_program: RPC) -> Option<RPCReply> {
    eprintln!("RPC_METHOD_EXPORT: {:?}", rpc_program);
    Some(RPCReply::Export(Mount::Export {}))
}

fn rpc_method_mnt(rpc_program: RPC) -> Option<RPCReply> {
    Some(RPCReply::Mnt(Mount::Mnt {}))
}

impl rpc::server::EventHandler for EventHandler {
    fn on_event(&self, name: &str, rpc_program: RPC) -> Option<RPCReply> {
        if self.callbacks.contains_key(name) == true {
            return self.callbacks[name](rpc_program)
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::{EventHandler, RPCReply};
    use crate::rpc::{RPC, RPCCall, Mount};
    use crate::rpc::server::EventHandler as RPCEventHandler;
    use crate::rpc::factories::generate_rpc_mount_procedure;

    fn test_method(rpc: RPC) -> Option<RPCReply> {
        Some(RPCReply::Export(Mount::Export {}))
    }

    #[test]
    fn it_will_call_callback() {
        let mut handler = EventHandler::new();
        let rpc_program = generate_rpc_mount_procedure();

        handler.add_callback("Testmethod".to_string(), test_method);
        assert_eq!(handler.on_event("Testmethod", rpc_program),
            Some(RPCReply::Export(Mount::Export {})));
    }
}
