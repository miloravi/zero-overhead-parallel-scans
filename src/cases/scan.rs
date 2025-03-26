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
mod our_half_sized_blocks;
mod half_sized_blocks;
mod half_sized_variant;
mod no_lookback_chained;
mod unchanged_half_sized;


pub const SIZE: usize = 1024 * 1024 * 64;

pub fn run(cpp_enabled: bool) {
  for size in [SIZE] {
    let temp = chained::create_temp();
    let half_sized_temp = half_sized_blocks::create_temp(); //new temp for half_sized_blocks
    
    let input = unsafe { utils::array::alloc_undef_u64_array(size) };
    let output = unsafe { utils::array::alloc_undef_u64_array(size) };
    fill(&input);
    let name = "Prefix-sum (n = ".to_owned() + &(size).to_formatted_string(&Locale::en) + ")";
    benchmark(
        ChartStyle::WithKey,
        &name,
        || {},
        || { reference_sequential_single(&input, &output) }
      )
      .parallel("Unchanged half-sized", 3, None, false, || {}, |thread_count| {
        let task = unchanged_half_sized::init_single(&input, &half_sized_temp, &output);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .parallel("Chained scan", 4, None, false, || {}, |thread_count| {
        let task = chained::init_single(&input, &temp, &output);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .parallel("Half-sized blocks", 5, None, false, || {}, |thread_count| {
        let task = half_sized_blocks::init_single(&input, &half_sized_temp, &output);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .parallel("Half-sized variant", 6, None, false, || {}, |thread_count| {
        let task = half_sized_variant::init_single(&input, &half_sized_temp, &output);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .parallel("Adaptive chained scan", 7, None, true, || {}, |thread_count| {
        let task = our_chained::init_single(&input, &temp, &output);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .parallel("Our Half-sized blocks", 8, None, true, || {}, |thread_count| {
        let task = our_half_sized_blocks::init_single(&input, &half_sized_temp, &output);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .parallel("No lookback chained scan", 9, None, true, || {}, |thread_count| {
        let task = no_lookback_chained::init_single(&input, &temp, &output);
        Workers::run(thread_count, task);
        compute_output(&output)
      })
      .cpp_sequential(cpp_enabled, "Reference C++", "scan-sequential", size)
      .cpp_tbb(cpp_enabled, "oneTBB", 1, None, "scan-tbb", size)
      .cpp_parlay(cpp_enabled, "ParlayLib", 2, None, "scan-parlay", size);
  }
}

pub fn run_inplace(cpp_enabled: bool) {
  for size in [SIZE] {
    let temp = chained::create_temp();
    let half_sized_temp = half_sized_blocks::create_temp();

    let values = unsafe { utils::array::alloc_undef_u64_array(size) };
    let name = "Prefix-sum inplace (n = ".to_owned() + &(size).to_formatted_string(&Locale::en) + ")";
    benchmark(
        if size < SIZE { ChartStyle::WithKey } else { ChartStyle::WithoutKey },
        &name,
        || { fill(&values) },
        || { reference_sequential_single(&values, &values) }
      )
      .parallel("Unchanged half-sized", 3, None, false, || { fill(&values) }, |thread_count| {
        let task = unchanged_half_sized::init_single(&values, &half_sized_temp, &values);
        Workers::run(thread_count, task);
        compute_output(&values)
      })
      .parallel("Chained scan", 4, None, false, || { fill(&values) }, |thread_count| {
        let task = chained::init_single(&values, &temp, &values);
        Workers::run(thread_count, task);
        compute_output(&values)
      })
      .parallel("Half-sized blocks", 5, None, false, || { fill(&values) }, |thread_count| {
        let task = half_sized_blocks::init_single(&values, &half_sized_temp, &values);
        Workers::run(thread_count, task);
        compute_output(&values)
      })
      // .parallel("Half-sized variant", 6, None, false, || { fill(&values) }, |thread_count| {
      //   let task = half_sized_variant::init_single(&values, &half_sized_temp, &values);
      //   Workers::run(thread_count, task);
      //   compute_output(&values)
      // })
      // .parallel("Adaptive chained scan", 7, None, true, || { fill(&values) }, |thread_count| {
      //   let task = our_chained::init_single(&values, &temp, &values);
      //   Workers::run(thread_count, task);
      //   compute_output(&values)
      // })
      // .parallel("Our Half-sized blocks", 8, None, true, || { fill(&values) }, |thread_count| {
      //   let task = our_half_sized_blocks::init_single(&values, &half_sized_temp, &values);
      //   Workers::run(thread_count, task);
      //   compute_output(&values)
      // })
      // .parallel("No lookback chained scan", 9, None, true, || { fill(&values) }, |thread_count| {
      //   let task = no_lookback_chained::init_single(&values, &temp, &values);
      //   Workers::run(thread_count, task);
      //   compute_output(&values)
      // })
      .cpp_sequential(cpp_enabled, "Reference C++", "scan-inplace-sequential", size)
      .cpp_tbb(cpp_enabled, "oneTBB", 1, None, "scan-inplace-tbb", size)
      .cpp_parlay(cpp_enabled, "ParlayLib", 2, None, "scan-inplace-parlay", size);
  }
}

pub fn fill(values: &[AtomicU64]) {
  for (idx, value) in values.iter().enumerate() {
    value.store(random(idx as u64) as u64, Ordering::Relaxed);
  }
}

pub fn compute_output(output: &[AtomicU64]) -> u64 {
  output[0].load(Ordering::Relaxed) + output[98238].load(Ordering::Relaxed) + output[output.len() - 123].load(Ordering::Relaxed) + output[output.len() - 1].load(Ordering::Relaxed)
}

pub fn reference_sequential_single(input: &[AtomicU64], output: &[AtomicU64]) -> u64 {
  scan_sequential(input, 0, output);
  compute_output(output)
}

pub fn scan_sequential(input: &[AtomicU64], initial: u64, output: &[AtomicU64]) -> u64 {
  let mut accumulator = initial;
  assert_eq!(input.len(), output.len());
  for i in 0 .. output.len() {
    accumulator += input[i].load(Ordering::Relaxed);
    output[i].store(accumulator, Ordering::Relaxed);
  }
  accumulator
}

pub fn fold_sequential(array: &[AtomicU64]) -> u64 {
  let mut accumulator = 0;
  for value in array {
    accumulator += value.load(Ordering::Relaxed);
  }
  accumulator
}

fn random(mut seed: u64) -> u32 {
  seed ^= seed << 13;
  seed ^= seed >> 17;
  seed ^= seed << 5;
  seed as u32
}
