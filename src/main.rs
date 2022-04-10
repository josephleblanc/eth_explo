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
use hex::FromHex;

use web3::types::{
    H160,
    H256, 
};

use eth_explo::{
    read_uniswap_tx,
    u256_to_f64,
    Trader,
    read_blocks,
    read_receipt,
    update_pools,
    update_liq_pools,
    Amm,
};
#[allow(non_snake_case)]

#[tokio::main]
async fn main() -> web3::Result<()> {
    let debug = false;
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

//    let debug_addr: Option<H160> = None;
    let mut is_debug_addr: bool;
    let debug_addr: Option<H160>= Some(H160::from_slice(
        &hex::decode(b"b7bb4454eb4ddca78c17b2911a0d0183984985b9").unwrap()));

    // Uniswap Address
    let uniswap_addr = H160::from_slice(
        &hex::decode(b"7a250d5630B4cF539739dF2C5dAcb4c659F2488D").unwrap());    

    // Set up trader tracking and uniswap tracking
    // uniswap_pools is: coin address, ratio to weth
    // initialize weth ratio to self as 1.0
    let mut uniswap_pools: HashMap<H160, f64> = HashMap::new();
    uniswap_pools.insert(weth_addr, 1.0_f64);
    let mut uniswap_liq: HashMap<H160, (f64, f64)> = HashMap::new();
    let mut trader_map: HashMap<H160, Trader> = HashMap::new();

    // Block range
    // Min saved is:
    // Max saved is: 14518566
    let start_block = 14508547_u64;
    let end_block = 14510000_u64;
    let n_blocks = end_block - start_block;

    // Track trades captured and missed by ignoring coins with no Weth pairing
    let mut captured_trade = 0;
    let mut missed_trade = 0;
    // Track shitcoin trades in which the uniswap pool would not have enough liquidity
    // to support a similar trade a 'shitcoin_threshold' number of times
    let mut shitcoin_trades = 0;
    let shitcoin_threshold = 1.0_f64;
    let mut shitcoin_list: Vec<H160> = vec![];

    let print_terminal = false;

    let mut receipts_missed = 0;
    for number in start_block..end_block {
        if print_terminal { println!("block {} of {}", number - start_block, n_blocks); }
//        println!("Scanning block {}     of  {}", number - start_block, n_blocks);
        let block_path = format!("../../testy/blocks/{}.json", number);
        let batched_blocks = read_blocks(&block_path).unwrap();

        'blocks: for tx in batched_blocks.transactions.iter()//.flat_map(|b| &b.transactions)
            .filter(|tx| tx.to.is_some() && tx.to.unwrap() == uniswap_addr &&
                    !tx.input.0.is_empty() &&
                    method_ids.contains(&hex::encode(&tx.input.0[0..4]).as_str()))
        {
            if debug_addr.is_some() && debug_addr.unwrap() == tx.from.unwrap() {
                is_debug_addr = true;
                println!("BLOCK FOR DEBUG ADDRESS: {}", number);
            } else { is_debug_addr = false; }
            if debug { println!("OK TX: {:?}", tx.hash); }
            let path_receipt = format!("../../testy/receipts/{}_{:?}.json", 
                                       tx.block_number.unwrap(), tx.hash);
            let receipt = read_receipt(path_receipt).ok();
            if receipt.is_none() {
                if is_debug_addr {println!("Missed for debug addr: {:?}", tx.hash);}
                receipts_missed += 1;
                continue
            }
            let receipt = receipt.unwrap();

            if receipt.logs.last().is_some() {
                if debug { println!("/tOK Receipt: {:?}", receipt.transaction_hash); }
                let extracted_uniswap = read_uniswap_tx(&tx, 
                                    &receipt, 
                                    &fid_vec, 
                                    &eth_for_ids,
                                    debug_addr.as_ref()).unwrap();
                let (start_token, start_amount, end_token, end_amount, 
                     _receiving_addr, pool_ratios) = &extracted_uniswap;
                let start_token = start_token.unwrap_or(weth_addr);

                // detect shitcoins and don't include traders or trades which cannot
                // be duplicated
                for ((coin0, coin1), (res0, res1)) in pool_ratios {
                    let is_shitcoin: bool = match (*coin0 == start_token,
                                                   *coin1 == start_token,
                                                   *coin0 == end_token.unwrap(),
                                                   *coin1 == end_token.unwrap()) {
                        (true,_,_,_) => *res0 < (shitcoin_threshold * start_amount),
                        (_,true,_,_) => *res1 < (shitcoin_threshold * start_amount),
                        (_,_,true,_) => *res0 < (shitcoin_threshold * end_amount),
                        (_,_,_,true) => *res1 < (shitcoin_threshold * end_amount),
                        _ => false
                    };
                    if is_shitcoin {
                        if is_debug_addr { 
                            println!("SHITCOIN DETECTED: {:?}", ((coin0, coin1), (res0, res1))); 
                        }
                        shitcoin_trades += 1;
                        shitcoin_list.push(match *coin0 == weth_addr {
                            true => *coin1,
                            false => *coin0,
                        });
                        continue 'blocks 
                    }
                    if is_debug_addr {println!("extracted_uniswap: {:?}", extracted_uniswap.clone());}
                }

                let mut trader = trader_map.entry(tx.from.unwrap())
                    .or_insert(Trader::new());
                trader.address = receipt.from;

                // only track coins which include a weth-coin pair
                if uniswap_pools.contains_key(&start_token)
                    || (uniswap_pools.contains_key(&start_token) 
                        && uniswap_pools.contains_key(&end_token.unwrap()))
                    || (start_token == weth_addr && uniswap_pools.contains_key(&end_token.unwrap())) {
                    captured_trade += 1;
                    trader.holdings.entry(start_token)
                        .and_modify(|cum_token_amt| *cum_token_amt =- start_amount)
                        .or_insert(-1.0 * start_amount); 
                    trader.holdings.entry(end_token.unwrap())
                        .and_modify(|cum_token_amt| *cum_token_amt += end_amount)
                        .or_insert(*end_amount);
                    trader.hist_cost += start_amount * uniswap_pools[&start_token];
                    trader.cum_gas += u256_to_f64(receipt.gas_used
                        .expect("every successful transaction requires gas"));
                    trader.cum_txs += 1_usize;
                } else {
                    missed_trade += 1;
                }
                    update_pools(&mut uniswap_pools, &pool_ratios, &weth_addr);
                    update_liq_pools(&mut uniswap_liq, &pool_ratios, &weth_addr);
            }
//            }

        }
    }
    for entry in &uniswap_pools {
        println!("{:?}", entry);
    }

    for entry in &uniswap_liq {
        println!("{:?}", entry);
    }

    let cloned_trader_map = trader_map.clone();
    let trader_coin_totals = cloned_trader_map.iter()
        .map(|(address, t)| (address, t.holdings.iter()
            .map(|(coin, amt)| (coin, match uniswap_pools.get(coin) {
                Some(ratio) => Some(amt * ratio),
                None => None }))
            .filter(|(_, amt)| (amt).is_some())
            .map(|(coin, amt)| (coin, amt.unwrap()))
            .collect::<Vec<(&H160, f64)>>()));

    for (address, holdings) in trader_coin_totals {
        if let Some(t) = trader_map.get_mut(&address) {
            t.total_assets = holdings.iter()
                .filter(|(_coin, amt)| amt > &0.0)
                .map(|(_coin, amt)| amt)
                .fold(0_f64, |acc, x| acc + x);
            t.total_debt = holdings.iter()
                .filter(|(_coin, amt)| amt < &0.0)
                .map(|(_coin, amt)| amt)
                .fold(0_f64, |acc, x| acc + x);
            t.profit_raw = t.total_assets + t.total_debt;
            t.profit_percent = -1.0 * t.total_assets / t.total_debt;
            t.roi_percent = (t.profit_raw + t.hist_cost - t.cum_gas) / t.hist_cost;
            t.real_gain_percent = (t.total_assets - t.cum_gas) / t.hist_cost;
//            println!("address: {}", address);
//            println!("\ttotal_assets: {}", t.total_assets);
//            println!("\ttotal_debt: {}", t.total_debt);
//            println!("\tprofit_percent: {}", t.profit_percent);
//            println!("\troi_percent: {}", t.roi_percent);
//            if t.profit_percent.is_nan() {
//                println!("\tholdings: {:?}", t.holdings);
//            }
        }
    }

    let mut trader_profit_list: Vec<(&H160, f64, f64, usize, f64, f64)> = trader_map.iter()
        .filter(|(_addr, t)| !t.roi_percent.is_nan())
        .map(|(addr, t)| (addr, t.roi_percent, t.profit_percent, t.cum_txs, t.profit_raw, t.real_gain_percent))
        .filter(|(_,_,_,cum_txs,_, _)| cum_txs > &10_usize)
        .collect();
    trader_profit_list.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    for entry in &trader_profit_list {
        println!("{:?}, {:.001}, {:.001}, {}, {}, {:.001}", entry.0, entry.1, entry.2, entry.3, entry.4, entry.5);
        if entry.1 > 2.0 {
            println!("\t{:?}", trader_map[entry.0].holdings);
        }
    }

    println!("receipts_missed = {}", receipts_missed);
    println!("trades captured: {}", captured_trade);
    println!("missed:          {}", missed_trade);
    println!("captured / total: {}", captured_trade as f64 / (captured_trade as f64 
                                                              + missed_trade as f64));
    println!("shitcoin_trades: {}", shitcoin_trades);
    Ok(())
}

