use std::fs;
use std::fs::File;
use std::io::prelude::*;
use eth_explo::{hex_to_u32, 
    decode_blockNumber,
    get_block_by_number
};

fn main() {
    // Endpoint file is in .gitignore to avoid sharing sensitive information
    // on github
    let endpoint = fs::read_to_string("../config/endpoint.txt")
        .unwrap();
    let endpoint: &str = endpoint.trim();
    println!("{:?}", endpoint);

//    let client = reqwest::Client::new();
//    let res: String = client.post(endpoint)
//        .body(
//r#"{
//    "jsonrpc": "2.0",
//    "method": "eth_getBlockByNumber",
//    "params": ["0xdc0ba9", true],
//    "id": "0"
//}"#
//        )
//        .send()
//        .unwrap()
//        .text()
//        .unwrap();
//
//    let res = format!("{}", res);

    let block_number = "0xdc0ba9";
    let hydrate = true;
    let res: String = String::new();
    let res = get_block_by_number(block_number, hydrate, endpoint, res);

    println!("response: {}", res);

    let mut file = File::create("hydrated.txt").unwrap();

    file.write_all(res.as_bytes()).unwrap();
}

