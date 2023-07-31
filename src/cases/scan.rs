use core::sync::atomic::{Ordering, AtomicU64};
use num_format::{Locale, ToFormattedString};
use crate::core::worker::*;
use crate::utils;
use crate::utils::benchmark::{benchmark, ChartStyle};

mod chained;
mod our_chained;
pub mod our_reduce_then_scan;
pub mod our_scan_then_propagate;
mod reduce_then_scan;
mod scan_then_propagate;

pub const SIZE: usize = 1024 * 1024 * 512;

pub fn run(cpp_enabled: bool) {
  for size in [SIZE / 8, SIZE] {
    let input = create_input(size);
    let mut temp = chained::create_temp();
    let output = unsafe { utils::array::alloc_undef_u64_array(size) };
    let name = "Prefix-sum (n = ".to_owned() + &(size).to_formatted_string(&Locale::en) + ")";
    benchmark(
        if size < SIZE { ChartStyle::WithKey } else { ChartStyle::WithoutKey },
        &name,
        || reference_sequential_single(&input, &output)
      )
      .parallel("Scan-then-propagate", 3, Some(13), false, |thread_count| {
        let task = scan_then_propagate::create_task(&input, &output);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .parallel("Reduce-then-scan", 5, None, false, |thread_count| {
        let task = reduce_then_scan::create_task(&input, &output);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .parallel("Chained scan", 7, None, false, |thread_count| {
        let task = chained::init_single(&input, &mut temp, &output);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .parallel("Assisted scan-t.-prop.", 2, Some(12), true, |thread_count| {
        let task = our_scan_then_propagate::create_task(&input, &output, None);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .parallel("Assisted reduce-t.-scan", 4, None, true, |thread_count| {
        let task = our_reduce_then_scan::create_task(&input, &output, None);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .parallel("Adaptive chained scan", 6, None, true, |thread_count| {
        let task = our_chained::init_single(&input, &temp, &output);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .cpp_sequential(cpp_enabled, "Reference C++", "scan-sequential", size)
      .cpp_parallel(cpp_enabled, "oneTBB", 1, None, "scan-tbb", size);
  }
}

pub fn create_input(size: usize) -> Box<[u64]> {
  (0..size).map(|x| random(x as u64) as u64).collect()
}

pub fn compute_output(output: &[AtomicU64]) -> u64 {
  output[0].load(Ordering::Relaxed) + output[98238].load(Ordering::Relaxed) + output[output.len() - 1].load(Ordering::Relaxed)
}

pub fn reference_sequential_single(input: &[u64], output: &[AtomicU64]) -> u64 {
  scan_sequential(input, 0, output);
  compute_output(output)
}

pub fn scan_sequential(input: &[u64], initial: u64, output: &[AtomicU64]) -> u64 {
  let mut accumulator = initial;
  assert_eq!(input.len(), output.len());
  for i in 0 .. output.len() {
    accumulator += input[i];
    output[i].store(accumulator, Ordering::Relaxed);
  }
  accumulator
}

pub fn fold_sequential(array: &[u64]) -> u64 {
  let mut accumulator = 0;
  for value in array {
    accumulator += value;
  }
  accumulator
}

fn random(mut seed: u64) -> u32 {
  seed ^= seed << 13;
  seed ^= seed >> 17;
  seed ^= seed << 5;
  seed as u32
}
