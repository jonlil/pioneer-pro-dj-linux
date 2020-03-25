use bytes::{Bytes, BytesMut};
use crate::rekordbox::DBMessage;
use crate::rekordbox::library::ClientState;

pub trait Controller {
    fn to_response(&self, request: RequestWrapper, context: &mut ClientState) -> Bytes;
}

pub struct RequestWrapper {
    pub message: DBMessage,
}

impl RequestWrapper {
    pub fn new(message: DBMessage) -> RequestWrapper {
        RequestWrapper { message: message }
    }

    pub fn to_response(self) -> BytesMut {
        self.message.to_response()
    }
}

pub struct RequestHandler<'a> {
    pub request: RequestWrapper,
    pub controller: Box<dyn Controller>,
    pub context: &'a mut ClientState,
}

impl <'a>RequestHandler<'a> {
    pub fn new(
        request_handler: Box<dyn Controller>,
        message: DBMessage,
        context: &'a mut ClientState
    ) -> RequestHandler<'a> {
        RequestHandler {
            request: RequestWrapper::new(message),
            controller: request_handler,
            context: context,
        }
    }

    pub fn respond_to(self) -> Bytes {
        self.controller.to_response(self.request, self.context)
    }
}
