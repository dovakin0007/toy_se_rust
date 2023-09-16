mod model;

use std::cmp::Ordering;
use std::str::{self, FromStr};
use std::fs::{ File, read_dir};
use std::path::Path;
use std::process::{ExitCode, exit};
use xml::reader::{EventReader, XmlEvent::Characters};
use std::result::Result;
use tiny_http::{Server, Response, Request, Method, Header};
use model::*;

fn _read_index_file(index_path: &str) -> Result<(), ()>{
    let index_file = File::open(index_path).unwrap();

    let tf_index_file:TermFreqIndex = serde_json::from_reader(index_file).unwrap();

    println!("{index_path}, contains {counts} files", counts = tf_index_file.len());
    Ok(())
}


fn read_xml_file<P: AsRef<Path>>(path: P) -> Result<String, ()>{
    let event_reader = EventReader::new(File::open(&path).map_err(|err|{
        eprintln!("ERROR: could not open file: {err}");
    })?);
    


    let mut content = String::new();
    for event in event_reader.into_iter(){
    
        match event{
            Ok(Characters(some_text)) =>{ content.push_str(&some_text);
            content.push_str(" "); 
            },
            Err(err) => eprintln!("{}", err),
            _ => () ,
        };

    }
    return Ok(content)
}

fn save_index_file(json_file_path: &str, tf_index: &TermFreqIndex) -> Result<(), ()>{
    let json_file =File::create(json_file_path).map_err(|err|{
        eprintln!("ERROR: Could not create file: {json_file_path} :{err} ");
    })?;

    serde_json::to_writer(json_file, &tf_index).map_err(|err|{
        eprintln!("ERROR: Could not serialize index to {json_file_path} :{err}");
    })?;
    Ok(())
}

fn tf_index_of_folder(dir_path: &Path, tf_index:&mut TermFreqIndex) -> Result<(), ()>{
    
   let dir = read_dir(dir_path).map_err(|err|{
    eprintln!("ERROR: could not open directory {dir_path} for indexing: {err}", dir_path = dir_path.display());
   }).unwrap();

   'next_file: for file in dir{
        
        let file = file.map_err(|err|{
            eprintln!("ERROR: could not open file {dir_path} for indexing: {err}", dir_path = dir_path.display());
        })?;

        let file_path = file.path();

        let file_type = file.file_type().map_err(|err|{
            eprintln!("ERROR : could not determine the type of file: {file_path}: {err}", file_path=file_path.display());
        })?;

        if file_type.is_dir(){
            tf_index_of_folder(&file_path, tf_index)?;
            continue 'next_file;
        }

        println!("Indexing {:?}...",&file_path);

        let content = match  read_xml_file(&file_path){
           Ok(content) => content.chars().collect::<Vec<_>>(),
            _ => continue 'next_file 
        };
        let mut tf = TermFreq::new();

        let token = Lexer::new(&content);
        for i in token{
            let term = i;

            match tf.get_mut(&term) {
                Some(freq) => {*freq += 1;},
                None => {tf.insert(term, 1);},
            }
    
        }
       
        let mut stats = tf.iter().collect::<Vec<_>>();
        stats.sort_by(|v, f|  f.1.cmp(v.1));
        tf_index.insert(file_path, tf);
    }
    Ok(())

}

fn usage(program: &str){
    eprintln!("Usage: {program} [SUBCOMMAND] [OPTIONS]");
    eprintln!("Subcommands");
    eprintln!("    index <folder> index the <folder> and save the index to index json file");
    eprintln!("    search <index-file> <query>     search <query> within the <index-file>");
    eprintln!("    serve <index-file> [address]    start local HTTP server with Web Interface");
}

pub fn serve_404(request: Request) -> Result<(),()> {
    request.respond(Response::from_string("404").with_status_code(404)).unwrap();
    Ok(())
}

pub fn server_static_file_request(request: Request, file_path:&str, content_type: &str) -> Result<(), ()>{
    let file_content = File::open(file_path).unwrap_or_else(|e| {
        eprintln!("ERROR: unable to open index.js: {}", e);
        exit(2)
        });
    let response = Response::from_file(file_content).with_header(
        content_type.parse::<tiny_http::Header>().unwrap()
    );
    request.respond(response).unwrap_or_else(|e| eprintln!("ERROR : Unable to send response {e}"));  
    Ok(())
} 

pub fn search_query<'a>(buffer: &'a str, tf_index: &'a TermFreqIndex) -> Vec<(&'a Path, f32)>{
    let mut relevant_doc: Vec<(&Path, f32)> = Vec::new();

    for (path, tf_table) in tf_index {
        let mut rank:f32 = 0f32;
        for token in Lexer::new(&buffer.chars().collect::<Vec<_>>()) {
            rank += tf(&token, tf_table) * idf(&token, tf_index);
        }
        relevant_doc.push((path, rank));
    }
    relevant_doc.sort_by(|(_, a ), (_, b)|
    {
        match (a.is_nan(), b.is_nan()) {
            (true, false) => Ordering::Less,   // a is NaN
            (false, true) => Ordering::Greater, // b is NaN
            (true, true) => Ordering::Equal,    // both are NaN (consider them equal)
            _ => a.partial_cmp(b).unwrap(),    // compare non-NaN values
        }
    }
    );
    relevant_doc.reverse();
    return relevant_doc;

}

pub fn get_response(mut request: Request, tf_index: &TermFreqIndex) -> Result<(), ()>{
    let mut buf = Vec::new();
            let _ = request.as_reader().read_to_end(&mut buf);
            let buffer = String::from_utf8( buf).unwrap();
            // println!("Body: {}", buffer.clone());

            let result = search_query(&buffer, tf_index);
            
            // for (path, tf) in result {
            //     println!("{path:?} => {tf}");
            // }

            let res =serde_json::to_string(&result.iter().take(20).collect::<Vec<_>>()).map_err(|e|{
                eprintln!("ERROR: could not convert search results to JSON: {e}")
            })?;
            request.respond(Response::from_string(res).with_header(Header::from_str("Content-Type: application/json").unwrap())).map_err(|err|{
                eprintln!("unable to send response : {err}")
            })?;
    Ok(())

}

fn tf(term: &str, doc: &TermFreq) -> f32 {
    let term_ap = (doc.get(term).cloned().unwrap_or(0)) as f32;
    let total_sum = (doc.clone().into_values().map(|v| v).sum::<usize>()) as f32;

    (term_ap/total_sum) as f32
}


fn idf(term: &str, d: &TermFreqIndex) -> f32 {
    let n = d.len() as f32;

    let m = d.values().filter(|tf| tf.contains_key(term)).count().max(1) as f32;

    return (n/m).log10();
}

pub fn serve_request(tf_index: &TermFreqIndex,mut request: Request) -> Result<(), ()>{
    println!("INFO: received request method: {:?}, url: {:?}", request.method(), request.url());

    match (request.method(), request.url()) {
        (Method::Post, "/api/search") => {
            get_response(request, tf_index).unwrap()
        }
        (Method::Get, "/index.js") => { 
            server_static_file_request(request, "index.js",  "Content-Type: text/javascript; charset=utf-8").unwrap()  
        }
        (Method::Get, "/") | (Method::Get, "/index.html") => {
            server_static_file_request(request, "index.html",  "Content-Type: text/html; charset=utf-8").unwrap()    
        }
        _ => serve_404(request).unwrap()
    }
    Ok(())   


}

fn entry() ->Result<(), ()> {
    let mut args = std::env::args();
    let program = args.next().expect("path to program is requestd");

    let subcommad = args.next().ok_or_else(||{
        usage(&program);
        eprintln!("ERROR: No subcommad specified")
    }).unwrap();

    match subcommad.as_str() {
        "index" => {
            let dir_path = args.next().ok_or_else(||{
                usage(&program);
                eprintln!("ERROR: no subcommand is been provided");
            }).unwrap();
            let new_dir_path = Path::new(&dir_path);
            let mut tf_index = TermFreqIndex::new();
            tf_index_of_folder(new_dir_path, &mut tf_index)?;
            save_index_file("index.json", &tf_index)?;
        },
        "search" => {
            todo!()
        }
        "serve"  => {
            let index_path = args.next().unwrap_or("index.json".to_string());
            
            let index_file = File::open(index_path).unwrap();

            let tf_index_file:TermFreqIndex = serde_json::from_reader(index_file).unwrap();
            let address = args.next().unwrap_or("127.0.0.1:8000".to_string());
            let server = Server::http(address.clone()).unwrap_or_else(|e| {
                eprintln!("Error connecting to server {e}");
                exit(1)
            });


            println!("Starting server at address {}", &address);
            for request in server.incoming_requests() {
                serve_request(&tf_index_file,request)?;
            }
        }     
         _ => {
            usage(&program);
            eprintln!("ERROR: unknown subcommand {subcommad}");
            
        }

    }
    Ok(())
}

pub fn main() -> ExitCode {
    match entry() {
        Ok(()) => ExitCode::SUCCESS,
        _=> ExitCode::FAILURE
    }
}