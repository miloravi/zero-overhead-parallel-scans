use core::sync::atomic::{Ordering, AtomicU64};
use crate::cases::scan::fold_sequential;
use crate::cases::scan::scan_sequential;
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workassisting_loop::*;

pub const SIZE: usize = crate::cases::scan::SIZE;
const BLOCK_SIZE: u64 = 1024 * 2; // half_size_blocks

// temp now twice as big, rounded up I guess, initialize everything as atomic 0
pub fn create_temp() -> Box<[BlockInfo]> {
  (0 .. (SIZE as u64 + BLOCK_SIZE - 1) / BLOCK_SIZE)
  .map(|_| BlockInfo{
    state: AtomicU64::new(STATE_INITIALIZED), 
    aggregate: AtomicU64::new(0), 
    prefix: AtomicU64::new(0)
  }).collect()
}

pub fn reset(temp: &[BlockInfo]) {
  for i in 0 .. temp.len() {
    temp[i].state.store(STATE_INITIALIZED, Ordering::Relaxed);
    temp[i].aggregate.store(0, Ordering::Relaxed);
    temp[i].prefix.store(0, Ordering::Relaxed);
  }
}

// Step 0: reset the atomics
pub fn init_single(input: &[AtomicU64], temp: &[BlockInfo], output: &[AtomicU64]) -> Task {
  // redundant, since it's the first one
  reset(temp);

  create_task(input, temp, output)
}

struct Data<'a> {
  input: &'a [AtomicU64],
  temp: &'a [BlockInfo],
  output: &'a [AtomicU64]
}

pub struct BlockInfo {
  pub state: AtomicU64,
  pub aggregate: AtomicU64,
  pub prefix: AtomicU64
}

pub const STATE_INITIALIZED: u64 = 0;
pub const STATE_AGGREGATE_AVAILABLE: u64 = 1;
pub const STATE_PREFIX_AVAILABLE: u64 = 2;

// Step 1: initialize the task
fn create_task(input: &[AtomicU64], temp: &[BlockInfo], output: &[AtomicU64]) -> Task {
  Task::new_dataparallel::<Data>(
    run, 
    finish, 
    Data{ input, temp, output },
    // is it not just temp.len(), always in every case?
    ((input.len() as u64 + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32, 
    false) // for "our" solution
}

// Step 2: run the task
fn run(_workers: &Workers, task: *const TaskObject<Data>, loop_arguments: LoopArguments) {
  let data = unsafe { TaskObject::get_data(task) }; // get the data from the task (unsafe is spooky)
  
  // Update this after every loop
  let mut unfinished_index: Option::<u32> = None;

  let mut unfinished_start = 0;
  let mut unfinished_end = 0;
  let mut unfinished_local = 0;


  // Parallel loop
  workassisting_loop!(loop_arguments, |block_index| {
    // reduce-then-scan, wat, this should be chained scan, not reduce-then-scan??
    let start = block_index as usize * BLOCK_SIZE as usize;
    let end = ((block_index as usize + 1) * BLOCK_SIZE as usize).min(data.input.len());

    if block_index == 0 {
      let local = scan_sequential(&data.input[start .. end], 0, &data.output[start .. end]);
      data.temp[block_index as usize].prefix.store(local, Ordering::Relaxed);
      data.temp[block_index as usize].state.store(STATE_PREFIX_AVAILABLE, Ordering::Release);
    } else {
      let local = fold_sequential(&data.input[start .. end]);
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

  // Perform last unfinished block
  if let Some(u_index) = unfinished_index {
    process_unfinished_block(data, u_index, unfinished_start, unfinished_end, unfinished_local);
  }
}

fn process_unfinished_block(data: &Data, u_index: u32, unfinished_start: usize, unfinished_end: usize, unfinished_local: u64) {
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

  // Make aggregate available of unfinished block
  data.temp[u_index as usize].prefix.store(aggregate + unfinished_local, Ordering::Relaxed);
  data.temp[u_index as usize].state.store(STATE_PREFIX_AVAILABLE, Ordering::Release);

  // Scan unfinished block
  scan_sequential(&data.input[unfinished_start .. unfinished_end], aggregate, &data.output[unfinished_start .. unfinished_end]);
}

fn finish(workers: &Workers, task: *mut TaskObject<Data>) {
  let _ = unsafe { TaskObject::take_data(task) };
  workers.finish();
}
