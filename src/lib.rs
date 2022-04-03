use web3::types::{
    Transaction,
    TransactionReceipt,
    H256,
    U256,
    H160,
    Log,
};

#[allow(dead_code)]
#[allow(unused)]

pub fn scrape_logs(logs: &Vec<Log>, fid_vec: &Vec<H256>, final_recipient: H256,
                   amount_out_min: U256)
// Required outputs:
// amount traded out, 
// pool ratios
-> (f64, f64, Vec<(f64, f64)>)
{
    let mut pool_ratios: Vec<(f64, f64)> = vec![];
    let mut end_amount = U256::from_big_endian(&[0_u8; 32]);
    let mut start_amount = U256::from_big_endian(&[0_u8; 32]);
    let mut approve = false;
    for log in logs {
        let function_hash = log.topics[0];
        if function_hash == fid_vec[5] {
            approve = true;
        } if function_hash == fid_vec[2] { // sync_fid
            let data_vec = get_bytes_vec(&log.data.0)
                .iter()
                .map(|entry| u256_to_f64(U256::from_big_endian(entry)))
                .collect::<Vec<f64>>();
            pool_ratios.push((data_vec[0], data_vec[1]));
        } else if function_hash == fid_vec[3] { // swap_fid
            let data_vec = get_bytes_vec(&log.data.0)
                .iter()
                .map(|entry| U256::from_big_endian(entry))
                .collect::<Vec<U256>>();
            let to = log.topics[2];
            if final_recipient == to {
                let amount_out = std::cmp::max(data_vec[2], data_vec[3]);
                end_amount = match amount_out > amount_out_min {
                    true => amount_out,
                    false => end_amount,
                };
            }
            if start_amount.low_u32() == 0_u32 {
                start_amount = std::cmp::max(data_vec[0], data_vec[1])
            }
        } else if function_hash == fid_vec[4] { // withdrawal_fid
            let src = log.topics[1];
            let wad: U256 = get_bytes_vec(&log.data.0)
                .iter()
                .map(|entry| U256::from_big_endian(entry))
                .last()
                .unwrap();
            if final_recipient == src || approve == true {
                end_amount = wad;
            }
        }
    }
    (u256_to_f64(start_amount), u256_to_f64(end_amount), pool_ratios)
}


// Method ID: 7ff36ab5
// Function: swapExactETHForTokens(uint256 amountOutMin, 
//                                  address[] path, 
//                                  address to, 
//                                  uint256 deadline)
// Method ID: fb3bdb41
// Function: swapETHForExactTokens(uint256 amountOut, 
//                                  address[] path, 
//                                  address to, 
//                                  uint256 deadline)
pub fn read_uniswap_tx(tx: &Transaction, receipt: &TransactionReceipt,
                                   fid_vec: &Vec<H256>, 
                                   short_input_funcs: &[&str])
-> Option<(Option<H160>,   //start_token
    f64,           // start_amount
    Option<H160>, // end_token
    f64,
    H160, //receiving_addr
    Vec<((H160, H160), (f64, f64))>)> // pool_ratios
{
    // exit early if no last log
    if receipt.logs.last().is_none() { 
        println!("REVERTED: Transaction has no logs, probably reverted.");
        return None; 
    } // exit early if no last log
    let method = hex::encode(&tx.input.0[0..4]);
    let is_eth_input = short_input_funcs.contains(&method.as_str());
    let input_offset = match is_eth_input {
        true => 0_usize,
        false => 1_usize,
    };

    let inputs_u8: Vec<&[u8]> = get_bytes_vec(&tx.input.0[4..]);
    let amount_out_min: U256 = match method.as_str() {
        // swapExactETHForTokens
        "7ff36ab5" 
        | "b6f9de95" => U256::from_big_endian(inputs_u8[0]), 
        "791ac947" 
        | "18cbafe5" 
        | "38ed1739" => U256::from_big_endian(inputs_u8[1]),
        _ => U256::from_dec_str(&"0").unwrap()
    };
    let end_token: Option<H160> = Some(H160::from_slice(&inputs_u8.last().unwrap()[12..]));
    let receiving_addr = H160::from_slice(&inputs_u8[2+input_offset][12..]);
    let start_token = match is_eth_input {
        true => None,
        false => Some(H160::from_slice(&inputs_u8[5+input_offset][12..])),
    };
   
    let raw_swap_addrs = &inputs_u8[5+input_offset..];
    
    // When ordering pairs, remember uniswap orders them by which by 
    // (lesser, greater) for (token_0, token_1)
    let swap_addrs = raw_swap_addrs.iter()
        .zip(raw_swap_addrs.iter()
             .skip(1))
        .map(|(token_0, token_1)| (H160::from_slice(&token_0[12..]),
                                   H160::from_slice(&token_1[12..])))
        .map(|(token_0, token_1)| match token_0 < token_1 {
            true => (token_0, token_1),
            false => (token_1, token_0),
            })
        .collect::<Vec<(H160, H160)>>();

    let final_recipient = H256::from_slice(inputs_u8[2+input_offset]);
    let (start_amount, end_amount, reserve_ratios) = 
        scrape_logs(&receipt.logs, fid_vec, final_recipient, amount_out_min);
    let pool_ratios = swap_addrs.into_iter()
        .zip(reserve_ratios)
        .collect::<Vec<((H160, H160), (f64, f64))>>();

    return Some((start_token, start_amount, end_token, end_amount, receiving_addr, pool_ratios)); 
}

//// Method ID: 791ac947
//// Function: swapExactTokensForETHSupportingFeeOnTransferTokens(
////                                                  uint256 amountIn, 
////                                                  uint256 amountOutMin, 
////                                                  address[] path, 
////                                                  address to, 
////                                                  uint256 deadline)
//pub fn print_swapExactTokensForETHSupportingFeeOnTransferTokens(tx: &Transaction, 
//                                      receipt: &TransactionReceipt,
//                                      fid_vec: &Vec<H256>) {
//// Outputs desired: 
//// Amount traded in, 
//// amount traded out, 
//// pool ratios, 
//// receiving address,
//// start token, 
//// end token
//    println!("swapExactTokensForETHSupportingFeeOnTransferTokens");
//    println!("tx hash: {:?}", tx.hash);
//    println!("methodId: {}", hex::encode(&tx.input.0[0..4]));
//    println!("from{:?}\nto: {:?}", tx.from, tx.to);
//    let inputs_u8 = get_bytes_vec(&tx.input.0[4..]);
//    println!("starting token: {}\namountIn tokens:{}",
//             H160::from_slice(&inputs_u8[6][12..]),
//             U256::from_big_endian(inputs_u8[0]));
//    println!("end token : {:?}", H160::from_slice(
//            &inputs_u8.last().unwrap()[12..]));
//    let last_log = match receipt.logs.last() { 
//        Some(logs) => logs.data.0.as_slice(),
//        None => return println!("REVERTED: No receipt log, probably reverted"),
//    };
//    let vec_last_log = get_bytes_vec(last_log)
//        .iter()
//        .map(|entry| U256::from_big_endian(entry))
//        .collect::<Vec<U256>>();
//    let mut final_recipient = H256::from_slice(inputs_u8[2]);
//    for log in &receipt.logs {
//        final_recipient = match print_decoded_log(log, fid_vec, &tx.from.unwrap(), final_recipient) {
//            Some(addr_U256) => addr_U256,
//            None => final_recipient,
//        };
//    }
//    println!("get_log_final_amount: {:?}", 
//             get_log_final_amount(&receipt.logs.iter().last().unwrap(), fid_vec));
//}
//
//// Method ID: b6f9de95
//// Function: swapExactETHForTokensSupportingFeeOnTransferTokens(
////                                          uint256 amountOutMin, 
////                                          address[] path, 
////                                          address to, 
////                                          uint256 deadline)
//pub fn print_swapExactETHForTokensSupportingFeeOnTransferTokens(tx: &Transaction,
//                                        receipt: &TransactionReceipt,
//                                        fid_vec: &Vec<H256>) {
//// Outputs desired: 
//// Amount traded in, 
//// amount traded out, 
//// pool ratios, 
//// receiving address,
//// start token, 
//// end token
//    println!("swapExactETHForTokensSupportingFeeOnTransferTokens");
//    println!("tx hash: {:?}", tx.hash);
//    println!("methodId: {}", hex::encode(&tx.input.0[0..4]));
//    println!("from{:?}\nto: {:?}", tx.from, tx.to);
//    let inputs_u8 = get_bytes_vec(&tx.input.0[4..]);
//    println!("Exact ETH in: {}\namountOutMin tokens:{}",
//             tx.value,
//             U256::from_big_endian(inputs_u8[0]));
//    println!("end token : {:?}", H160::from_slice(
//            &inputs_u8.last().unwrap()[12..]));
//    let last_log = match receipt.logs.last() { 
//        Some(logs) => logs.data.0.as_slice(),
//        None => return println!("REVERTED: No receipt log, probably reverted"),
//    };
//    let vec_last_log = get_bytes_vec(last_log)
//        .iter()
//        .map(|entry| U256::from_big_endian(entry))
//        .collect::<Vec<U256>>();
//    for log in &receipt.logs {
//        let final_recipient = H256::from_slice(inputs_u8[2]); // different than normal [2]
//        print_decoded_log(log, fid_vec, &tx.from.unwrap(), final_recipient);
//    }
//    println!("get_log_final_amount: {:?}", 
//             get_log_final_amount(&receipt.logs.iter().last().unwrap(), fid_vec));
//
//}
pub fn get_bytes_vec(inputs: &[u8]) -> Vec<&[u8]> {
    let index_inputs = inputs.len() / 32;
    
    let mut vec_index: Vec<&[u8]> = vec![];
    for index in (1..=index_inputs).map(|i| i*32) {
        vec_index.push(&inputs[(index-32)..index]);
    }
    vec_index
}


fn poor_man_log_10(mut n: U256) -> usize {
    let mut places: usize = 0;
    let ten = U256::from(10_u64);
    while !n.is_zero() {
        n = n.checked_div(ten).unwrap();
            places += 1;
    }
    places
}

fn highest_five(mut n: U256) -> (U256, usize) {
    let places = poor_man_log_10(n);
    if places >= 16 {
        let ten_k = U256::exp10(places - 16);
        n = n.checked_div(ten_k).unwrap();
    }
    (n, places)
}

pub fn u256_to_f64(n: U256) -> f64 {
    let (top_five, places) = highest_five(n);
    match places >= 16 {
        true => (top_five.as_usize() as f64) * 10_f64.powf(places as f64 - 16.0),
        false => top_five.as_usize() as f64
    }
}

//pub fn coin_to_weth(start_amount: U256, weth_val: U256, coin_val: U256) -> f64 {
//    start_amount(
//}
