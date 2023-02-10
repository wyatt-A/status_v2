use crate::stage::Stage;
use serde::{Serialize,Deserialize};
use crate::status::Status;

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct Request {
    pub stage:Stage,
    pub big_disk:Option<String>,
    pub run_number_list:Vec<String>,
}

impl Request {
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).expect("cannot serialize request")
    }
}

#[derive(Serialize,Deserialize,Debug,Clone)]
pub enum ServerError {
    RequestParse,
    BIGGUS_DISKUS_NotSet,
}

#[derive(Serialize,Deserialize,Debug,Clone)]
pub enum Response {
    Error(ServerError),
    Success(Status)
}