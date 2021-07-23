Simulates transaction handling on a list of clients.



### General approach


### Assumptions


### Multithreaded architecture





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

#### Multithreaded transactions reader

On ryzen 3600 (6 cores / 12 threads):
```
[2021-07-22T22:36:33Z INFO  paytoy::bench] MTReader read 1000000 records in 144.0002ms 6.9444 millions/second
```

We get a nice speedup of approx 6 times.

Running the full application:

Single client:

```
[2021-07-22T22:39:52Z INFO  paytoy::bench] Multi-threaded application time: 320.164ms 3.1234 millions/second
```

Multiple clients:
```
[2021-07-22T22:41:07Z INFO  paytoy::bench] Multi-threaded application time: 360.2897ms 2.7755 millions/second
```

Thus an overall speedup of approx x3 or x4 depending on the dataset.

#### Can we do better?

#### Multithreaded processing

For an asset with multiple clients, we can assign each thread a subset of clients for processing:

```
[2021-07-22T23:00:06Z INFO  paytoy::bench] Single threaded application time: 1.0790833s 0.9267 millions/second
[2021-07-22T23:00:06Z INFO  paytoy::bench] Multi-threaded application time: 189.9495ms 5.2645 millions/second
```

A whooping ~5.7x speedup


For a dataset with a single client:

```
[2021-07-22T23:02:03Z INFO  paytoy::bench] Single threaded application time: 1.064819s 0.9391 millions/second
[2021-07-22T23:02:03Z INFO  paytoy::bench] Multi-threaded application time: 328.4573ms 3.0445 millions/second
```

Note that much of a speedup, since only a single thread manages a single client. But still, around x3 speedup

Moved to hashbrown HashMap, additional improvements:
Single client:

```
[2021-07-22T23:20:06Z INFO  paytoy::bench] Single threaded application time: 984.3992ms 1.0158 millions/second
[2021-07-22T23:20:06Z INFO  paytoy::bench] Multi-threaded application time: 269.338ms 3.7128 millions/second
```

Multiple clients:
```
[2021-07-22T23:23:39Z INFO  paytoy::bench] Single threaded application time: 1.043596s 0.9582 millions/second
[2021-07-22T23:23:39Z INFO  paytoy::bench] Multi-threaded application time: 172.0427ms 5.8125 millions/second
```

Effectively a 6x improvement (utilizing all 6 physical cores)

