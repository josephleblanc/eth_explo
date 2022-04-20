# eth_explo
The goal of the program is to provide metrics on trader profit and loss by using on-chain analysis of uniswap v2 swaps.
This is a tool to explore uniswap transactions by analyzing onchain data requested from an archive ethereum node api.
This project assumes you already have the transactions and receipts filtered and downloaded to a local folder.

To use the tool, go into the main.rs file and edit the start_block and end_block numbers to select the range of blocks
you would like to analyze. The program will iterate over the transactions in a block and use the tx hash to lookup a
local receipt related to the transaction. It will then save the transaction data to a struct called Trader, and later
derive some basic statistics about the change in value of the Trader's investments over the duration of the blocks provided.

The program creates a trader profile which tracks the profit and loss of all addresses used in uniswap trades by creating
a portfolio for each coin swapped, providing a negative value for coins traded in and a positive value for coins traded out.
Once all blocks have been processed, the trader's final investment portfolio is valued by simulating a swap from all coins
in the portfolio for eth at the current swap rate for the coin using uniswap's constant product formula.

Uniswap trades are parsed by reading the block transaction data and receipt logs. A built-in parser will identify whether
a trade is, for example, a swap of the form swapExactETHForTokens or swapETHForExactTokens. Because successful uniswap
transactions will update the pool ratio for any given coin pair after the swap is determined to be valid, it is possible
to keep an historical ratio of pool reserves and simulate the liquidation of a given trader's portfolio. This simulated
liquidation takes into account uniswap protocol fees and the constant product formula used by the uniswap v2 smart contract.

Future updates may include parsers for uniswap v3 or other DEXs such as sushiswap or pancake swap.

This project may be useful in developing a wider model of user behavior in transactions across the chain by incorporating
transactions between EOAs and known public addresses of CEXs.
