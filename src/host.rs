use std::net::TcpStream;
use regex::Regex;
use ssh_rs::{LocalSession, LocalShell, ssh};
use crate::request::{Request, Response, ServerError};

pub struct Host {
    hostname:String,
    user_name:String,
    port:u32,
    session:LocalSession<TcpStream>,
    shell:LocalShell<TcpStream>
}

#[derive(Debug)]
pub enum ConnectionError {
    UnableToConnect,
    UnableToStartShell,
}

impl Host {
    pub fn new(hostname: &str, user_name: &str, port: u32) -> Result<Self, ConnectionError> {
        let private_key = std::env::home_dir().expect("cannot get home dir").join(".ssh").join("id_rsa");
        let mut session = ssh::create_session()
            .username(user_name)
            .private_key_path(private_key)
            .connect(&format!("{}:{}", hostname, port)).map_err(|_| ConnectionError::UnableToConnect)?
            .run_local();
        let mut shell = session.open_shell().map_err(|_| ConnectionError::UnableToStartShell)?;

        Ok(Host {
            hostname: hostname.to_string(),
            user_name: user_name.to_string(),
            port,
            session,
            shell,
        })
    }

    pub fn submit_request(&mut self,request:&Request) -> Response {

        // get some workstation env variable like WKS_BIN or something ...
        // use that to call binary with server sub-command

        let req_string = serde_json::to_string(request).expect("unable to serialize request");
        let command_string = format!("server --request={}\n",req_string);
        self.shell.write(command_string.as_bytes()).expect(&format!("unable to write to shell on {}",self.hostname));

        let mut string_response = String::new();

        let json = loop {
            let byte_chunk = match self.shell.read(){
                Err(_) => break None,
                Ok(bytes) => bytes
            };
            let string_buffer = String::from_utf8(byte_chunk).unwrap();
            string_response.push_str(&string_buffer);

            // check that string_response contains the json
            let re = Regex::new(r"\|\|(.*)\|\|").expect("incorrect regular expression");

            let txt = string_response.as_str();

            let capture = re.captures(txt);

            match capture {
                Some(cap) => {
                    break Some(cap.get(1).expect("no group captured").as_str());
                }
                None => {

                }
            }
        };

        match json {
            Some(json) => serde_json::from_str(json).expect("cannot deserialize response"),
            None => Response::Error(ServerError::RequestParse)
        }

    }

}