# eth_explo
Explores the ethereum blockchain

The goal of the program is to provide metrics on trader profit and loss by using on-chain analysis.
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
