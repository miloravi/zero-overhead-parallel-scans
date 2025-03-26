use core::sync::atomic::{Ordering, AtomicU64, AtomicUsize};
use crate::cases::compact::{compact_sequential, count_sequential};
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workassisting_loop::*;
use crate::cases::compact::unchanged_half_sized::{ Data, BlockInfo, reset, BLOCK_SIZE, STATE_PREFIX_AVAILABLE, STATE_AGGREGATE_AVAILABLE, STATE_INITIALIZED };

pub fn create_task(mask: u64, input: &[u64], temp: &[BlockInfo], output: &[AtomicU64], output_count: &AtomicUsize) -> Task {
  reset(temp);
  Task::new_dataparallel::<Data>(run, finish, Data{ mask, input, temp, output, output_count }, ((input.len() as u64 + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32, false)
}

fn run(_workers: &Workers, task: *const TaskObject<Data>, loop_arguments: LoopArguments) {
  let data = unsafe { TaskObject::get_data(task) };
    
  // Update this after every loop
  let mut unfinished_index: Option::<u32> = None;
  let mut unfinished_start = 0;
  let mut unfinished_end = 0;
  let mut unfinished_local = 0;

  let mut sequential = true;
  workassisting_loop!(loop_arguments, |block_index| {
    // Local scan
    // reduce-then-scan
    let start = block_index as usize * BLOCK_SIZE as usize;
    let end = ((block_index as usize + 1) * BLOCK_SIZE as usize).min(data.input.len());

    // Check if we already have an aggregate of the previous block.
    // If that is the case, then we can perform the scan directly.
    // Otherwise we perform a reduce-then-scan over this block.
    let aggregate_start = if !sequential {
      None // Don't switch back from parallel mode to sequential mode
    } else if block_index ==  0 {
      Some(0)
    } else {
      let previous = block_index - 1;
      let previous_state = data.temp[previous as usize].state.load(Ordering::Acquire);
      if previous_state == STATE_PREFIX_AVAILABLE {
        Some(data.temp[previous as usize].prefix.load(Ordering::Acquire))
      } else {
        None
      }
    };

    if let Some(aggregate) = aggregate_start {
      let local = compact_sequential(data.mask, &data.input[start .. end], data.output, aggregate);
      data.temp[block_index as usize].prefix.store(local, Ordering::Relaxed);
      data.temp[block_index as usize].state.store(STATE_PREFIX_AVAILABLE, Ordering::Release);
    } else {
      sequential = false;
      let local = count_sequential(data.mask, &data.input[start .. end]);
      // Share own local value
      data.temp[block_index as usize].aggregate.store(local, Ordering::Relaxed);
      data.temp[block_index as usize].state.store(STATE_AGGREGATE_AVAILABLE, Ordering::Release);

      // Check if it has an unfinished block
      if let Some(u_index) = unfinished_index {
        process_unfinished_block(data, u_index, unfinished_start, unfinished_end, unfinished_local);
      }

      // Replace unfinished block with current block
      unfinished_index = Some(block_index);

      unfinished_start = start;
      unfinished_end = end;
      unfinished_local = local;
    }
  });

    // perform last unfinished block
    if let Some(u_index) = unfinished_index {
      process_unfinished_block(data, u_index, unfinished_start, unfinished_end, unfinished_local);
    }
}

fn process_unfinished_block(data: &Data, u_index: u32, unfinished_start: usize, unfinished_end: usize, unfinished_local: usize) {
  // Find aggregate
  let mut aggregate = 0;
  let mut previous = u_index - 1;

  loop {
    let previous_state = data.temp[previous as usize].state.load(Ordering::Acquire);
    if previous_state == STATE_PREFIX_AVAILABLE {
      aggregate = data.temp[previous as usize].prefix.load(Ordering::Acquire) + aggregate;
      break;
    } else if previous_state == STATE_AGGREGATE_AVAILABLE {
      aggregate = data.temp[previous as usize].aggregate.load(Ordering::Acquire) + aggregate;
      previous = previous - 1;
    } else {
      // Continue looping until the state of previous block changes.
    }
  }

  // Make aggregate available
  data.temp[u_index as usize].prefix.store(aggregate + unfinished_local, Ordering::Relaxed);
  data.temp[u_index as usize].state.store(STATE_PREFIX_AVAILABLE, Ordering::Release);
  compact_sequential(data.mask, &data.input[unfinished_start .. unfinished_end], data.output, aggregate as usize);
}

fn finish(workers: &Workers, task: *mut TaskObject<Data>) {
  let data = unsafe { TaskObject::take_data(task) };
  let count = data.temp[data.temp.len() - 1].prefix.load(Ordering::Relaxed);
  data.output_count.store(count, Ordering::Relaxed);
  workers.finish();
}
