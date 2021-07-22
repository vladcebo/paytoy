Simulates transaction handling on a list of clients.


### Basic Benchmarking:

#### Compare how long it takes to read a raw file vs to read it and parse using csv and serde

```
[2021-07-22T18:47:09Z DEBUG paytoy::bench] Time to read the raw file: 27.924ms
[2021-07-22T18:47:09Z DEBUG paytoy::transactions_reader] STBulkReader reading the transactions
[2021-07-22T18:47:11Z DEBUG paytoy::transactions_reader] Read 1000000 records in 1.3908061s. Throughput: 0.7190074 millions/second
```

Results: CSV Serde is much slower ~50 times

Conclusion: IO is a huge bottleneck and has to be optimized

Full application with transaction processing on a single thread:


Single client:

```
[2021-07-22T21:24:05Z INFO  paytoy::bench] Single threaded application time: 1.0651176s 0.9389 millions/second
```
Multiple clients:
```
[2021-07-22T21:22:55Z INFO  paytoy::bench] Single threaded application time: 1.6521117s 0.6053 millions/second
```

Conclusion:

Because of the second hashmap and we have to keep track of clients as well, it's longer to process transactions for multiple clients.
But, different clients can be parallelized independetely.
