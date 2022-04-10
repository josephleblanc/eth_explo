use web3::types::{
    Transaction,
    TransactionReceipt,
    H256,
    U256,
    H160,
    Log,
    Block
};

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::{Duration, Instant};
use std::thread;
use std::fmt::Debug;

use std::collections::HashMap;

#[allow(dead_code)]
#[allow(unused)]

#[derive(Debug, Clone)]
pub struct Trader {
    pub address: H160,
    pub total_assets: f64,
    pub total_debt: f64,
    pub cum_gas: f64,
    pub cum_txs: usize,
    pub profit_raw: f64,
    pub hist_cost: f64,
    pub holdings: HashMap<H160, f64>,
    pub profit_percent: f64,
    pub roi_percent: f64,
    pub real_gain_percent: f64,
}

impl Trader {
    pub fn new() -> Trader {

        Trader {
            address: H160::from_low_u64_be(0_u64),
            total_assets: 0_f64,
            total_debt: 0_f64,
            cum_gas: 0_f64,
            cum_txs: 0,
            profit_raw: 0_f64,
            hist_cost: 0_f64,
            holdings: HashMap::new(),
            profit_percent: 0_f64,
            roi_percent: 0_f64,
            real_gain_percent: 0_f64,
        }
    }
}



pub fn scrape_logs(logs: &Vec<Log>, fid_vec: &Vec<H256>, final_recipient: H256,
                   amount_out_min: U256,
                   debug_addr: Option<&H160>,
                   start_addr: &H160
                   )
// Required outputs:
// amount traded out, 
// pool ratios
-> (f64, f64, Vec<(f64, f64)>)
{
    let mut pool_ratios: Vec<(f64, f64)> = vec![];
    let mut end_amount = U256::from_big_endian(&[0_u8; 32]);
    let mut start_amount = U256::from_big_endian(&[0_u8; 32]);
    let mut approve = false;
    if debug_addr.is_some() && debug_addr.unwrap() == start_addr {
        println!("Entering scrape_logs");
    }
    for log in logs {
//        match debug_addr {
//            Some(addr) => match addr == start_addr {
//                true => println!("{}", serde_json::to_string_pretty(log).unwrap()),
//                false => (),
//            },
//            None => ()
//        };

        let function_hash = log.topics[0];
        if function_hash == fid_vec[5] {
            approve = true;
        } if function_hash == fid_vec[2] { // sync_fid
            let data_vec = get_bytes_vec(&log.data.0)
                .iter()
                .map(|entry| u256_to_f64(U256::from_big_endian(entry)))
                .collect::<Vec<f64>>();
            if debug_addr.is_some() && debug_addr.unwrap() == start_addr { 
                println!("SYNC:\n\tpool_ratios.push(({:?}, {:?}))", data_vec[0], data_vec[1]);
            }
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
            if debug_addr.is_some() && debug_addr.unwrap() == start_addr {
                println!("SWAP:\n\tend_amount: {}", end_amount);
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
            if debug_addr.is_some() && debug_addr.unwrap() == start_addr {
                println!("WITHDRAWAL: final_recipient == src is {}\n\twad = {}\n\tend_amount = {}", final_recipient == src, wad, end_amount);
            }
        }
    }
    if debug_addr.is_some() && debug_addr.unwrap() == start_addr {
        println!("Exiting scrape_logs");
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
                                   short_input_funcs: &[&str],
                                   debug_addr: Option<&H160>)
-> Option<(Option<H160>,   //start_token
    f64,           // start_amount
    Option<H160>, // end_token
    f64,
    H160, //receiving_addr
    Vec<((H160, H160), (f64, f64))>)> // pool_ratios
{
    if debug_addr.is_some() && debug_addr.unwrap() == &tx.from.unwrap() {
        println!("Entering read_uniswap_tx");
    }
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
        if debug_addr.is_some() && debug_addr.unwrap() == &tx.from.unwrap() {
            println!("swap_addrs: {:?}", swap_addrs);
        }

    let final_recipient = H256::from_slice(inputs_u8[2+input_offset]);
    let (start_amount, end_amount, reserve_ratios) = 
        scrape_logs(&receipt.logs, fid_vec, final_recipient, amount_out_min,
                    debug_addr,
                    &tx.from.unwrap());
    let pool_ratios = swap_addrs.into_iter()
        .zip(reserve_ratios)
        .collect::<Vec<((H160, H160), (f64, f64))>>();

    if debug_addr.is_some() && debug_addr.unwrap() == &tx.from.unwrap() {
        println!("pool ratios output: {:?}", pool_ratios);
        println!("Exiting read_uniswap_tx");
    }

    return Some((start_token, start_amount, end_token, end_amount, 
                 receiving_addr, pool_ratios));
}

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

pub fn read_blocks<P: AsRef<Path>>(path: P) 
-> Result<Block<Transaction>, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let u = serde_json::from_reader(reader)?;

    Ok(u)
}

pub fn read_receipt<P: AsRef<Path>>(path: P)
-> Result<TransactionReceipt, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let u = serde_json::from_reader(reader)?;

    Ok(u)
}

pub fn time_check(start_t: Instant) {
    println!("Time check: {:?}", start_t.elapsed());
    let one_sec = Duration::from_millis(1000);
    if start_t.elapsed() < one_sec {
        println!("nap time: {:?}", one_sec - start_t.elapsed());
        thread::sleep(one_sec - start_t.elapsed());
    }
}

pub fn debug_print<T: Debug>(to_print: T) -> bool {
    println!("DEBUG PRINT");
    println!("\t{:?}", to_print);
    true
}

pub fn update_pools(uniswap_pools: &mut HashMap<H160, f64>,
                    pool_ratios: &Vec<((H160, H160), (f64, f64))>,
                    weth_addr: &H160) -> () {
    for (coins, values) in pool_ratios { // try only saving weth tuples
        let updated_pool = match coins.0 == *weth_addr {
            true => Some((coins.1, values.0 / values.1)),
            false => match coins.1 == *weth_addr {
                true => Some((coins.0, values.1 / values.0)),
                false => None,
            }
        };
//        println!("\tupdating pool with: {:?}", updated_pool);
        match updated_pool {
            Some((coin, ratio_weth)) => uniswap_pools.insert(coin, ratio_weth),
            None => None,
        };
    }
}

pub fn update_liq_pools(uniswap_liq: &mut HashMap<H160, (f64, f64)>,
                    pool_ratios: &Vec<((H160, H160), (f64, f64))>,
                    weth_addr: &H160) -> () {
    for (coins, values) in pool_ratios { // try only saving weth tuples
        let updated_pool = match coins.0 == *weth_addr {
            true => Some((coins.1, (values.0, values.1))),
            false => match coins.1 == *weth_addr {
                true => Some((coins.0, (values.1, values.0))),
                false => None,
            }
        };
//        println!("\tupdating pool with: {:?}", updated_pool);
        match updated_pool {
            Some((coin, reserves)) => uniswap_liq.insert(coin, reserves),
            None => None,
        };
    }
}

#[derive(Debug)]
pub struct Amm {
    token0_name: H160,
    token1_name: H160,
    token0_amt: f64,
    token1_amt: f64,
    const_product: f64
}

impl Amm {
    pub fn new(token0_name: H160, token1_name: H160,
               token0_amt: f64, token1_amt: f64,
               const_product: f64) -> Amm {
        Amm {
            token0_name,
            token1_name,
            token0_amt,
            token1_amt,
            const_product,
        }
    }
    pub fn swap(&mut self, token_in: H160, amt_in: f64) -> f64 {
        let (res_in, res_out): (&mut f64, &mut f64) = match token_in == self.token0_name {
            true => (&mut self.token0_amt, &mut self.token1_amt),
            false => match token_in == self.token1_name {
                true => (&mut self.token1_amt, &mut self.token0_amt),
                false => panic!("invalid token input") }
        };
        *res_in += amt_in;
        let amt_out = *res_out - (self.const_product / *res_in);
        *res_out -= amt_out;
        amt_out
    }
    pub fn immut_swap(&self, token_in: H160, amt_in: f64) -> f64 { 
        let (res_in, res_out): (&f64, &f64) = match token_in == self.token0_name {
            true => (&self.token0_amt, &self.token1_amt),
            false => match token_in == self.token1_name {
                true => (&self.token1_amt, &self.token0_amt),
                false => panic!("invalid token input") }
        };
        *res_out - (self.const_product / (*res_in + amt_in))
    }
}
