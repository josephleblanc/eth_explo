// Important information to extract:
//      Keeping track of trade:
//          x Amount traded in
//          x Amount traded out
//          x Uniswap pool ratio at time of exchange
//          x Sending address
//          x Receiving address (if WETH, make sure it counts as a coin they own)
//          x Bool sending addr == receiving addr
//          x Start token
//          x End token
//
//      Trader profile:
//          Trade frequency
//          Total amount in
//          Total amount out
//          Percentage profit
//          
//      Interesting Traders Profile
//          Use API to get total trade history
//
//      Uniswap pools
//          Pool addr for each token
//          New pool ratio after each trade

use std::collections::HashMap;
use std::fmt::Debug;
use std::thread;
use std::time::{Instant, Duration};
use std::env;

use hex::FromHex;

use web3::types::{H160, BlockId, U64, H256};

use eth_explo::{
    read_uniswap_tx,
    u256_to_f64,
    Trader
};
//    print_swapETHForExactTokens, // Leaving these here for testing
//    print_swapExactTokensForETH, //
//    print_swapExactTokensForTokens, //
//    print_swapTokensForExactTokens, //
//    print_swapExactTokensForETHSupportingFeeOnTransferTokens, //
//    print_swapExactETHForTokensSupportingFeeOnTransferTokens, //
//    };
#[allow(non_snake_case)]

#[tokio::main]
async fn main() -> web3::Result<()> {
    dotenv::dotenv().ok();

    // Set method ids for input functions, hash of first 8 hex digits in keccak hash
    let swapExactETHForTokens = "7ff36ab5";
    let swapETHForExactTokens = "fb3bdb41";

    let eth_for_ids: [&str; 2] = [
        swapExactETHForTokens,
        swapETHForExactTokens
    ];

    let swapExactTokensForTokens = "38ed1739";
    let swapExactTokensForETH = "18cbafe5";
    let swapTokensForExactTokens = "8803dbee";
    let swapExactTokensForETHSupportingFeeOnTransferTokens = "791ac947";
    let method_ids: [&str; 6] = [
        swapExactETHForTokens,
        swapETHForExactTokens,
        swapExactTokensForTokens,
        swapExactTokensForETH,
        swapTokensForExactTokens,
        swapExactTokensForETHSupportingFeeOnTransferTokens
    ];
    // Set function IDs for functions called in logs.
    let deposit_fid = H256::from_slice(Vec::from_hex(
            "e1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c").unwrap()
            .as_slice());
    let transfer_fid = H256::from_slice(Vec::from_hex(
            "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef").unwrap()
            .as_slice());
    let sync_fid = H256::from_slice(Vec::from_hex(
            "1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1").unwrap()
            .as_slice());
    let swap_fid = H256::from_slice(Vec::from_hex(
            "d78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822").unwrap()
            .as_slice());
    let withdrawal_fid = H256::from_slice(Vec::from_hex(
            "7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65").unwrap()
            .as_slice());
    let approval_fid = H256::from_slice(Vec::from_hex(
            "8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925").unwrap()
            .as_slice());
    let fid_vec = vec![deposit_fid, 
                        transfer_fid, 
                        sync_fid, 
                        swap_fid, 
                        withdrawal_fid,
                        approval_fid];

    // Eth does not have an address, so just use WETH address instead
    let weth_addr = H160::from_slice(Vec::from_hex(
        "c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2").unwrap()
            .as_slice());

    let debug_address = H160::from_slice(Vec::from_hex(
        "34ed8d7b93485b454a57f856c54be08ade3c9132").unwrap()
        .as_slice());

//    // Use this address for value at time of trade, calculated in WETH
//    let hist_value_addr = H160::from_slice(Vec::from_hex(
//            "0000000000000000000000000000000000000000").unwrap()
//            .as_slice());

    // Set up web3 endpoint
    let uniswap_addr = H160::from_slice(
        &hex::decode(b"7a250d5630B4cF539739dF2C5dAcb4c659F2488D").unwrap());    
    let transport = web3::transports::http::Http::new(
        &env::var("ARCHIVE_NODE").unwrap())
        .unwrap();
    let web3 = web3::Web3::new(transport);

    // Set up trader tracking and uniswap tracking
    // uniswap_pools is: coin address, ratio to weth
    let mut uniswap_pools: HashMap<H160, f64> = HashMap::new();
    let mut trader_map: HashMap<H160, Trader> = HashMap::new();

    // Time stuff
    let mut start_t = Instant::now();
    let one_sec = Duration::from_millis(1_000);
    let mut call_count = 0;

    // Block range
    let start_block = 14503000_u64;
    let end_block = 14503100_u64;
    let n_blocks = end_block - start_block;
    
    for number in start_block..end_block {
        println!("Scanning block {}     of  {}", number - start_block, n_blocks);
        let block_number = BlockId::from(U64::from(number));
        let block_txs = web3.eth().block_with_txs(block_number).await?
            .unwrap()
            .transactions;
        call_count += 1;
        println!("call_count: {}/10", call_count);
        for tx in block_txs.iter() {
            if tx.to.is_some() && tx.to.unwrap() == uniswap_addr { // test for last_log somewhere here
                let method_id = hex::encode(&tx.input.0[0..4]);
                if method_ids.contains(&method_id.as_str()) {
                    // Find transactions with the uniswap router using the commands in method_ids
                    // update trader hashmap
                    // update uniswap hashmap
                    let receipt = web3.eth().transaction_receipt(tx.hash).await?.unwrap();
                    call_count += 1;
//                    println!("call_count: {}/10", call_count);
//                    println!("method_id: {}", method_id.as_str());
//                    println!("{:?}", tx.hash);
                    if receipt.logs.last().is_some() {
                    let extracted_uniswap = read_uniswap_tx(tx, 
                                        &receipt, 
                                        &fid_vec, 
                                        &eth_for_ids).unwrap();
                    let (start_token, start_amount, end_token, end_amount, receiving_addr, pool_ratios) = extracted_uniswap;
//                    println!(
//"start_token:{:?}\nstart_amount:{:?}\nend_token:{:?}
//end_amount:{:?}\nreceiving_addr:{:?}\npool_ratios:{:?}",
//start_token, start_amount, end_token, end_amount, receiving_addr, pool_ratios
//                    );
//                    println!("send_addr == receiving_addr: {}", tx.from.unwrap() == receiving_addr);
                    let mut trader = trader_map.entry(tx.from.unwrap())
                        .or_insert(Trader::new());
                    let start_token = start_token.unwrap_or(weth_addr);
//                    println!("start_token assigned to weth: {}", weth_addr);

                    if uniswap_pools.contains_key(&start_token) 
                        || (start_token == weth_addr && uniswap_pools.contains_key(&end_token.unwrap())) {
                        trader.holdings.entry(start_token)
                            .and_modify(|cum_token_amt| *cum_token_amt =- start_amount)
                            .or_insert(-1.0 * start_amount); 
                        trader.holdings.entry(end_token.unwrap())
                            .and_modify(|cum_token_amt| *cum_token_amt += end_amount)
                            .or_insert(end_amount);
                        trader.hist_cost += start_amount * uniswap_pools[&start_token];
                        trader.cum_gas -= u256_to_f64(receipt.gas_used
                            .expect("every successful transaction requires gas"));
                        trader.cum_txs += 1_usize;
                    }
                    update_pools(&mut uniswap_pools, &pool_ratios, &start_amount, &weth_addr);

                    } else {
//                        println!("REVERTED: NO LOG DATA, PROBABLY REVERTED");
                    }
                }
//                println!("");
            }
            // Rate limiting
            if call_count >= 9 {
                println!("call_count = {}/10", call_count);
                let time_passed = start_t.elapsed();
                println!("TIME CHECK: time passed = {} millis", time_passed.as_millis());
                if time_passed < one_sec {
                    println!("NAP TIME: {:?}", one_sec - time_passed);
                    thread::sleep(one_sec - time_passed);
                }
                start_t = Instant::now();
                call_count = 0;
                println!("call_count: {}/10", call_count);
            }
        }
    }
    for entry in &uniswap_pools {
        println!("{:?}", entry);
    }
//    for entry in &uniswap_pools {
//        println!("uniswap_pools");
//        println!("{:?}", entry);
//    }

    let mut trader_profit_list: Vec<(&H160, f64)> = trader_kv.iter()
        .map(|(address, portfolio)| (address, portfolio.iter() 
             .filter_map(|(coin, amt)| debug_print(coin).then(|| match uniswap_pools.get(coin) {
                         Some(ratio) => Some(amt * ratio),
                         None => None }).unwrap()) 
             .fold(0_f64, |acc, x| acc + x)))
        .collect::<Vec<(&H160, f64)>>();
    trader_profit_list.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
//    for entry in &trader_profit_list {
//        println!("{:?}", entry);
//    }

    println!("RAW PROFIT");
    let mut trader_profit_percent = trader_profit_list
        .iter()
        .filter(|(addr, _)| trader_hist_cost.contains_key(addr))
        .map(|(addr, profit)| (addr, profit / trader_hist_cost[addr]))
        .filter(|(_, percent)| percent.is_normal())
        .collect::<Vec<(&&H160, f64)>>();
    trader_profit_percent.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    println!("\nPROFIT BY PERCENT");
    for entry in trader_profit_percent {
        println!("{:?}", entry);
    }
    Ok(())
}

fn debug_print<T: Debug>(to_print: T) -> bool {
    println!("DEBUG PRINT");
    println!("\t{:?}", to_print);
    true
}

pub fn update_pools(uniswap_pools: &mut HashMap<H160, f64>,
                    pool_ratios: &Vec<((H160, H160), (f64, f64))>,
                    start_amount: &f64,
                    weth_addr: &H160) -> () {
    for (coins, values) in pool_ratios { // try only saving weth tuples
        let updated_pool = match coins.0 == *weth_addr {
            true => Some((coins.0, start_amount * values.0 / values.1)),
            false => match coins.1 == *weth_addr {
                true => Some((coins.1, start_amount * values.1 / values.0)),
                false => None,
            }
        };
        match updated_pool {
            Some((coin, ratio_weth)) => uniswap_pools.insert(coin, ratio_weth),
            None => None,
        };
    }
}
