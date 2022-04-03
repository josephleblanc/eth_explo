// Important information to extract:
//      Keeping track of trade:
//          x Amount traded in
//          x Amount traded out
//          Uniswap pool ratio at time of exchange
//          x Sending address
//          x Receiving address (if WETH, make sure it counts as a coin they own)
//          Bool sending addr == receiving addr
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
    u256_to_f64
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
    let mut trader_kv: HashMap<H160, HashMap<H160, f64>> = HashMap::new();
    let mut uniswap_pools: HashMap<(H160, H160), (f64, f64)> = HashMap::new();
    let mut trader_hist_cost: HashMap<H160, f64> = HashMap::new();

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
                    let (start_token, start_amount, end_token, end_amount, receiving_addr, pool_ratios) = 
                        read_uniswap_tx(tx, 
                                        &receipt, 
                                        &fid_vec, 
                                        &eth_for_ids).unwrap();
//                    println!(
//"start_token:{:?}\nstart_amount:{:?}\nend_token:{:?}
//end_amount:{:?}\nreceiving_addr:{:?}\npool_ratios:{:?}",
//start_token, start_amount, end_token, end_amount, receiving_addr, pool_ratios
//                    );
//                    println!("send_addr == receiving_addr: {}", tx.from.unwrap() == receiving_addr);
                    let from_addr = H160::from(tx.from.expect("Every tx should have this field").0);
                    let start_token = start_token.unwrap_or(weth_addr);
//                    println!("start_token assigned to weth: {}", weth_addr);

                    let trader_entry = trader_kv.entry(from_addr).or_insert(HashMap::new());
                    *trader_entry.entry(start_token)
                        .or_insert(-1.0 * start_amount) 
                        -= start_amount;
                    let gas_fee_f64 = u256_to_f64(receipt.gas_used
                                        .expect("every successful transaction requires gas"));
                    *trader_entry.entry(weth_addr)
                        .or_insert(-1.0 * gas_fee_f64)
                        -= gas_fee_f64;
                    *trader_entry.entry(end_token.unwrap())
                        .or_insert(end_amount)
                        += end_amount;

                    if tx.from == Some(debug_address) {
                        println!("TRANSACTION\n{:?}", tx.from);
                    }

                    for (coin_pair, value_pair) in pool_ratios { // try only saving weth tuples
                        if coin_pair.0 == weth_addr || coin_pair.1 == weth_addr {
                            uniswap_pools.insert(coin_pair, value_pair);
//                            println!("Inserting to uniswap_pools: {:?}", (coin_pair, value_pair));

                            let hist_val = match coin_pair.0 == weth_addr {
                                true => (end_amount * value_pair.0 / value_pair.1) + gas_fee_f64,
                                false => (end_amount * value_pair.1 / value_pair.0) + gas_fee_f64,
                            };
//                            println!("hist_val: {}", hist_val);
                            *trader_hist_cost.entry(tx.from.expect("All valid transactions have a sender"))
                                .or_insert(hist_val)
                                += hist_val;
                        }
                    }

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
    // Calculate the ratio of 
    let uniswap_ratios: HashMap<&H160, f64> = uniswap_pools.iter()
        .filter_map(|((coin0, coin1), (res0, res1))| match &weth_addr == coin0 {
            true => Some((coin1, *res0 / *res1)),
            false => match coin1 == &weth_addr {
                true  => Some((coin0, *res1 / *res0)),
                false => None,}})
        .collect();
//    for entry in &uniswap_ratios {
//        println!("uniswap_ratios");
//        println!("{:?}", entry);
//    }

    let mut trader_profit_list: Vec<(&H160, f64)> = trader_kv.iter()
        .map(|(address, portfolio)| (address, portfolio.iter() 
             .filter_map(|(coin, amt)| debug_print(coin).then(|| match uniswap_ratios.get(coin) {
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
