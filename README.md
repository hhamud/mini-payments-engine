# mpe


## Introduction
This is a mini payments engine that:
- Reads a series of transactions from a CSV
- Updates the clients accounts
- Handles disputes and Chargebacks
- Outputs the state of the clients accounts as a CSV


## Installation and running

``` sh
Cargo run -- transactions.csv
```
This will run the payments engine with the input supplied and then produce the results into the stdout.


``` sh
Cargo run -- transactions.csv > accounts.csv
```
Same result as above but this time it will produce the results as a csv file.


