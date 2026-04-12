use binrw::{binread, binwrite};

use crate::protocol::{
    Error,
    Exception,
    r#struct::{Readable, Writable},
};

/// Request Protocol Data Unit.
#[binwrite]
#[bw(big)]
#[derive(Copy, Clone)]
pub struct Request<const FUNCTION_CODE: u8, T: Writable> {
    #[bw(calc = FUNCTION_CODE)]
    function_code: u8,

    pub data: T,
}

/// Response Protocol Data Unit.
#[binread]
#[br(big)]
#[derive(Copy, Clone)]
pub enum Response<const FUNCTION_CODE: u8, T: Readable> {
    Ok {
        #[br(temp, assert(function_code == FUNCTION_CODE))]
        function_code: u8,

        data: T,
    },
    Exception {
        #[br(temp, assert(function_code == FUNCTION_CODE | 0x80))]
        function_code: u8,

        exception: Exception,
    },
}

impl<const FUNCTION_CODE: u8, T: Readable> From<Response<FUNCTION_CODE, T>> for Result<T, Error> {
    fn from(response: Response<FUNCTION_CODE, T>) -> Self {
        match response {
            Response::Ok { data, .. } => Ok(data),
            Response::Exception { exception, .. } => Err(Error::Exception(exception)),
        }
    }
}
