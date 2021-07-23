Simulates transaction handling on a list of clients.
The application is benchmarked o a Ryzen 5 3600, with 6 cores and 12 threads.


### Assumptions

In the application we have the following assumptions:
* Records in the csv file are in the correct (as described in the requirements) format
* A record that cannot be parsed if the requirement above doens't hold is ignored
* Records come from a single, chronologically ordered stream (it can be a from a file, network etc.). It can be extended to multiple concurrent streams, but then the consitency and relative chronological order of transactions in different streams shall be handled
* Any transaction on a locked account is ignored


### Main workflow

* Started as test driven development to pass the simple cases
* A single threaded application is built and verified first
* The application is benchmarked, the bottlenecks are investigated and optimized into a multithreaded application
* Tests are written for large datasets (millions of records), with a single client and multiple clients since the performance differs for those

### Multithreaded architecture

The processing pipelined dataflow is based on the results of benchmarks that are run on large datasets (see below).

![alt text](data_flow.svg)

1) The file reader reads blocks from the sequentially disk (we assume it's we have a single disk so the IO cannot be parallized, in any case, file reading is not the bottleneck)
2) The blocks are dispatched on a thread pool that does the parsing of raw byte blocks into lists of transaction records using csv and serde.
3) Since we do that in parallel and the chronological order matters, a reorder thread receives lists of transactions and reorders them in chronological order, obtaining a stream (iterator) over all transactions.
4) A dispatcher reads the tarnsactions from the stream and dispatches them to a thread pool for processing. Each thread in that pool manages for simplicity a fixed subset of clients. Thus, if only one client is present in the dataset, then only one thread will work on it (since sequential consistency of applying transactions to an account really matters)

### Final results for benchmarking

The number of records is 1 million. Reported values are in millions of transactions per second and rounded to the first decimal point

ST = single threaded

MT = multithreaded

#### Reader
reader | Single threaded | Multithreaded
--- | --- | ---
single client | 2.0| 10.0
all clients | 2.0 | 10.0

Thus we get a speed-up of about **x5** on my machine (can be finely tuned depending on the number of cores)


### Full application

When running the benchmark on the application, the stdout reporting stage is omitted.

application | Single threaded | Multithreaded
--- | --- | ---
single client | 1.6 | 4.7
all clients | 1.5 | 5.7

Thus if we have a single client, then we do not parallelize the transactions processing and we have a speedup of around **x3**. For multiple accounts the speedup is around **x4**.


### Basic Benchmarking:

#### Compare how long it takes to read a raw file vs to read it and parse using csv and serde

```
[2021-07-22T18:47:09Z DEBUG paytoy::bench] Time to read the raw file: 27.924ms
[2021-07-22T18:47:09Z DEBUG paytoy::transactions_reader] STBulkReader reading the transactions
[2021-07-22T18:47:11Z DEBUG paytoy::transactions_reader] Read 1000000 records in 1.3908061s. Throughput: 0.7190074 millions/second
```

**Results**: CSV Serde is much slower ~50 times

**Conclusion**: IO is not the bottleneck, serialization and parsing is.
