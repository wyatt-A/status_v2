use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use clap::{Command, Parser};
use clap;
use ssh_config::SSHConfig;
use dirs;
use status_v2::host::Host;
use status_v2::pipe::ConfigCollection;
use status_v2::request::{Request, Response, ServerError};
use status_v2::status::Status;

#[derive(clap::Parser,Debug)]
struct Args {
    #[clap(subcommand)]
    action: Action
}

#[derive(clap::Subcommand,Debug)]
enum Action {
    Client(ClientArgs),
    Server(ServerArgs),
}


#[derive(clap::Args,Debug)]
struct ClientArgs {
    runno_list:Vec<String>,
    last_pipeline:String,
    #[clap(short, long)]
    big_disk:Option<Vec<String>>,
    #[clap(short, long)]
    pipe_configs:Option<PathBuf>,
}

#[derive(clap::Parser,Debug,Clone,Serialize,Deserialize)]
pub struct ServerArgs {
    pub request_json:String,
}


fn main(){

    // parse arguments
    let args:Args = Args::parse();

    match &args.action {
        Action::Client(args) => run_client(args),
        Action::Server(args) => run_server(args)
    }


}

fn run_client(args:&ClientArgs){

    let this_exe = std::env::current_exe().expect("cannot determine this program!");

    let home_dir:PathBuf = dirs::home_dir().expect("cannot get home directory!");
    let this_host = utils::computer_name();

    // load pipe configs
    let pipe_config_dir = args.pipe_configs.clone().unwrap_or(home_dir.join(".pipe_configs"));

    let conf_col = ConfigCollection::from_dir(&pipe_config_dir);

    // get list of hosts from last_pipe
    let needed_servers = conf_col.servers(&args.last_pipeline);

    // parse big_disk option
    let big_disks = match &args.big_disk {
        Some(args) => {
            let mut big_disks = HashMap::<String,String>::new();
            for arg in args {
                //let arg = arg.to_owned();
                let split:Vec<&str> = arg.split(":").collect();
                if split.len()  != 2 {
                    panic!("BIGGUS_DISKUS must contain a : for")
                }
                big_disks.insert(split[0].to_string(),split[1].to_string());
            }
            Some(big_disks)
        }
        None => None
    };

    // load ssh config and check for existence
    let ssh_config_file = home_dir.join(".ssh").join("config");
    if !ssh_config_file.exists(){
        println!("no ssh config found! You need to set this up first!");
        return
    }

    // load ssh config file and parse it to a usable type
    let config_str = utils::read_to_string(&ssh_config_file,"");
    let c = SSHConfig::parse_str(&config_str).unwrap();

    // check for a config for each server
    for server in &needed_servers {
        let server_config = c.query(server);
        if server_config.is_empty(){
            println!("we didn't find a ssh config for {} in your .ssh/config file. Please add the host the the file!",server);
            return
        }
    }


    // connect to servers

    let mut ssh_connections = HashMap::<String,Host>::new();

    for server in &needed_servers {
        let server_config = c.query(server);
        let username = server_config.get("User");
        match username {
            Some(user) => {
                match Host::new(server,user,22) {
                    Err(_) => println!("unable to connect to {}. Make sure you have password-less access! You may need to run ssh-copy-id {}@{}",server,user,server),
                    Ok(host) => {
                        ssh_connections.insert(server.to_string(),host);
                    }
                };
            }
            None => {
                println!("we didn't find a username for {}. Please specify the username in .ssh/config",server);
                return
            }
        }
    }


    // loop thru stages in pipe to get status
    // if stage is incomplete and a pipe, recurse, append to status report


    let pipe = conf_col.get_pipe(&args.last_pipeline).unwrap();
    let preferred_computers = pipe.preferred_computer.clone().unwrap_or(vec![]);

    for stage in &pipe.stages {

        let mut request = Request{
            stage: stage.clone(),
            big_disk:None,
            run_number_list:args.runno_list.clone(),
        };


        // append to preferred computers
        match &stage.preferred_computer {
            Some(computers) => {
                for computer in computers {
                    preferred_computers.push(computer.clone());
                }
            }
            None => {}
        }

        if preferred_computers.is_empty() {
            // do local check
            let mut cmd = std::process::Command::new(&this_exe);
            cmd.args(vec!["server",&request.to_json()]);
            let o = cmd.output().expect("failed to launch");
            let resp_str = String::from_utf8(o.stdout);



        }else {
            // do remote check
            for computer in preferred_computers {
                let mut host = ssh_connections.get_mut(&computer).expect("host not found!");
                let big_disk = match &big_disks {
                    Some(disks) => {
                        match disks.get(&computer) {
                            Some(disk) => Some(disk.to_owned()),
                            None => None
                        }
                    }
                    None => None
                };
                request.big_disk = big_disk;
                let resp = host.submit_request(&request);
            }
        }

    }
}

fn run_server(args:&ServerArgs){

    let re = match process_request(&args.request_json) {
        Err(e) => Response::Error(e),
        Ok(stat) => Response::Success(stat)
    };
    let resp_string = serde_json::to_string(&re).expect("unable to serialize response");
    print!("||{}||",resp_string);


    fn process_request(req:&str) -> Result<Status,ServerError> {
        let req:Request = serde_json::from_str(req).map_err(|_|ServerError::RequestParse)?;
        let big_disk = match &req.big_disk {
            Some(str) => str.to_string(),
            None => std::env::var("BIGGUS_DISKUS").map_err(|_|ServerError::BIGGUS_DISKUS_NotSet)?
        };
        let status = req.stage.file_check(&big_disk,&req.run_number_list,None);
        Ok(status)
    }
}