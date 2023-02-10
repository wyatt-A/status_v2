use regex::Regex;
use serde::{Serialize,Deserialize};
use crate::status::{Status, StatusType};

#[derive(Serialize,Deserialize,Debug,Clone)]
pub enum SignatureType {
    Discrete,
    ManyToOne,
    ManyToMany,
    OneToMany,
    OneToOne,
}

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct Stage {
    pub label:String,
    pub preferred_computer:Option<Vec<String>>,
    #[serde(with = "serde_regex")]
    pub completion_file_pattern:Regex,
    pub directory_pattern:String,
    pub signature_type:SignatureType,
    pub required_file_keywords:Option<Vec<String>>,
}

impl Stage {
    pub fn file_check(&self, big_disk:&str, runno_list:&Vec<String>, base_runno:Option<String>) -> Status {
        use SignatureType::*;

        println!("running file check ...");

        //println!("stage label: {}",self.label);

        let re = Regex::new(r"(\$\{[[:alnum:]_]+\})").unwrap();


        // may have user arg BIGGUS_DISKUS
        // it an optional hostname : alternate_biggus
        // when it has no hostname, OR hostname matches this host, we use it.

        let mut the_dir = self.directory_pattern.clone();
        re.captures_iter(&self.directory_pattern).for_each(|captures|{
            for cap_idx in 1..captures.len(){
                if captures[cap_idx].eq("${BIGGUS_DISKUS}") {
                    the_dir = the_dir.replace(&format!("{}",&captures[cap_idx]),&big_disk);
                }else if captures[cap_idx].eq("${PARAM0}") {
                    //todo(deal with 0 match?)
                    the_dir = the_dir.replace(&format!("{}",&captures[cap_idx]),&runno_list[0]);
                }else if captures[cap_idx].eq("${PREFIX}"){
                    the_dir = the_dir.replace(&format!("{}",&captures[cap_idx]),"diffusion");
                }
                else if captures[cap_idx].eq("${SUFFIX}"){
                    the_dir = the_dir.replace(&format!("{}",&captures[cap_idx]),"");
                }
                else if captures[cap_idx].eq("${PROGRAM}"){
                    the_dir = the_dir.replace(&format!("{}",&captures[cap_idx]),"dsi_studio");
                }
                else if captures[cap_idx].eq("${BASE}"){
                    the_dir = the_dir.replace(&format!("{}",&captures[cap_idx]),&base_runno.clone().unwrap_or("BASE_RUNNO".to_string()));
                }
                else if captures[cap_idx].eq("${SEP}"){
                    the_dir = the_dir.replace(&format!("{}",&captures[cap_idx]),"");
                }else {
                    panic!("capture not recognized")
                }
            }
        });

        // directory should be valid now... if its missing we cant check?... or this stage has 0 progress
        // return not started?

        println!("\tresolved directory pattern :{:?}",the_dir);

        // trim required matches based on signature type=
        let required_matches = match &self.signature_type {
            ManyToMany => {
                runno_list.clone()
            }
            ManyToOne => {
                // we will assume only the base runno is involved in the match
                vec![base_runno.expect("base runno must be specified for ManyToOne signature type").to_string()]
            }

            OneToMany => {
                self.required_file_keywords.clone().expect("you need to specify required file keywords for OneToMany signature pattern")
            }

            OneToOne => {
                // relies on completion file pattern to filter to the ONLY thing which should match.
                vec![".".to_string()]
            }

            _=> {
                panic!("signature not implemented")
            }

        };

        let contents = match std::fs::read_dir(&the_dir) {
            Err(_) => return Status{
                label: self.label.clone(),
                progress: StatusType::NotStarted,
                children: vec![]
            },
            Ok(contents) => contents
        };

        let mut included = vec![];
        for thing in contents {
            let tp = thing.unwrap();
            let file_name = tp.path().file_name().unwrap().to_string_lossy().into_owned();
            let file_path = tp.path().to_string_lossy().into_owned();
            if self.completion_file_pattern.is_match(&file_path) {
                included.push(file_name)
            }
        }

        println!("\t\tconsidered items: {:?}",included);
        println!("\t\tpattern: {:?}",self.completion_file_pattern);

        included.sort();
        let glob = included.join("/");

        let mut count = 0;

        for rm in &required_matches {
            if Regex::new(&format!("(^|/).*{}.*($|/)",rm)).unwrap().is_match(&glob){
                count = count + 1;
                //println!("{}",rm);
            }
        }

        return if count == required_matches.len() {
            Status{
                label: self.label.clone(),
                progress: StatusType::Complete,
                children: vec![]
            }
        } else if count == 0 {
            Status{
                label: self.label.clone(),
                progress: StatusType::NotStarted,
                children: vec![]
            }
        } else {
            Status{
                label: self.label.clone(),
                progress: StatusType::InProgress(count as f32 / required_matches.len() as f32),
                children: vec![]
            }
        }
    }
}