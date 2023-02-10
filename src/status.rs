use serde::{Serialize,Deserialize};

#[derive(Debug,Serialize,Deserialize,Clone,Copy)]
pub enum StatusType {
    InProgress(f32),
    NotStarted,
    Complete,
    Invalid,
}

impl StatusType {
    pub fn to_float(&self) -> f32 {
        match &self {
            StatusType::Invalid => panic!("invalid status detected!"),
            StatusType::Complete => 1.0,
            StatusType::NotStarted => 0.0,
            StatusType::InProgress(prog) => *prog
        }
    }
}

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct Status {
    pub label:String,
    pub progress:StatusType,
    pub children:Vec<Status>
}