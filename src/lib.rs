pub fn hex_to_u32(h: &str) -> u32 {
    println!("h: {}", h);
    h.chars()
        .rev()
        .enumerate()
        .fold(0, |acc, (i, x)| acc + 
              ((16_u32.pow(i as u32) ) * (x.to_digit(16).unwrap() as u32)))
}

pub fn decode_blockNumber(res: &str) -> u32 {
    let block_number = res
        .split(['"', '\\', ',', ':', '{', '}'])
        .filter(|item| !item.is_empty())
        .last()
        .unwrap();

    let block_number = &block_number[2..];
    println!("block_number: {}", block_number);

    hex_to_u32(block_number)
}

pub fn get_block_by_number(block_number: &str, hydrate: bool, endpoint: &str, mut output: String)
-> String {
    let hydrate = match hydrate {
        true => "true",
        false => "false",
    };
    let client = reqwest::Client::new();
    let payload = format!(
"{{
    \"jsonrpc\": \"2.0\",
    \"method\": \"eth_getBlockByNumber\",
    \"params\": [\"{}\", {}],
    \"id\": \"0\"
}}",
        block_number, hydrate);
    output = client.post(endpoint)
        .body(payload)
        .send()
        .unwrap()
        .text()
        .unwrap();

    output
}
