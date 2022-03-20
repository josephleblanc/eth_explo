use std::fs;

fn main() {
    let endpoint = fs::read_to_string("../config/endpoint.txt")
        .unwrap();
    let endpoint: &str = endpoint.trim();
    println!("{:?}", endpoint);

    let client = reqwest::Client::new();
    let res: String = client.post(endpoint)
        .body(
r#"{
    "jsonrpc": "2.0",
    "method": "eth_blockNumber",
    "params": [],
    "id": "0"
}"#
        )
        .send()
        .unwrap()
        .text()
        .unwrap();

    let res = format!("{}", res);


    println!("response: {}", res);

}

