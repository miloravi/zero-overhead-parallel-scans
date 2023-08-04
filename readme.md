# Zero-Overhead Parallel Scans for CPUs

This repository contains implementations of three parallel scan algorithms for multi-core CPUs, which do not need to fix the number of available
cores at the start and have zero overhead compared to sequential scans when executed on a single core. These two
properties are in contrast with most existing work and are important when the parallelism of scans is mixed with task
parallelism. We achieve this by adapting the classic three-phase scan algorithms, and these adaptations also exhibit
better performance than the original algorithms on many cores. Furthermore, we adapt the chained scan with decoupled
look-back algorithm to also have these two properties. While this algorithm was originally designed for GPUs, we show
it is also suitable for multi-core CPUs, outperforming the classic three-phase scans in our benchmarks. Our
adaptation of the three-phase reduce-then-scan algorithm is in most cases even faster, and is in general the fastest parallel scan on any
number of threads.

The scan algorithms are implemented using the [work-assisting scheduler](https://github.com/ivogabe/workassisting).

## Instructions
To run the benchmarks, the Rust compiler and cargo need to be installed. Furthermore gnuplot needs to be installed, as the benchmark code automatically generates charts of the results. The benchmarks can be run with `cargo run`. Depending on the processor, it may be needed to tune `AFFINITY_MAPPING` in `src/utils/thread_pinning.rs`. This specifies the order in which the cores of the processor are used.

The program will ask if the C++ and oneTBB implementations should be enabled. This requires Linux, g++, cmake and git. When enabled, it will automatically download and install oneTBB locally.
