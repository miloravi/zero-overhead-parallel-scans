use core::sync::atomic::{Ordering, AtomicU64};
use crate::cases::scan::fold_sequential;
use crate::cases::scan::scan_sequential;
use crate::cases::scan::half_sized_blocks::{ BlockInfo, reset, STATE_PREFIX_AVAILABLE, STATE_AGGREGATE_AVAILABLE };
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workassisting_loop::*;

const BLOCK_SIZE: u64 = 1024 * 2; // half sized blocks?

// with only 1 thread?
pub fn init_single(input: &[AtomicU64], temp: &[BlockInfo], output: &[AtomicU64]) -> Task {
  reset(temp);
  create_task(input, temp, output)
}

//pub const STATE_INITIALIZED: u64 = 0;
// pub const STATE_AGGREGATE_AVAILABLE: u64 = 1;
// pub const STATE_PREFIX_AVAILABLE: u64 = 2;

struct Data<'a> {
  input: &'a [AtomicU64],
  temp: &'a [BlockInfo],
  output: &'a [AtomicU64]
}

fn create_task(input: &[AtomicU64], temp: &[BlockInfo], output: &[AtomicU64]) -> Task {
  Task::new_dataparallel::<Data>(run, finish, Data{ input, temp, output }, ((input.len() as u64 + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32, false)
}

fn run(_workers: &Workers, task: *const TaskObject<Data>, loop_arguments: LoopArguments) {
  let data = unsafe { TaskObject::get_data(task) };
  let mut sequential = true;

  // Update this after every loop
  let mut unfinished_index: Option::<u32> = None;

  // Maybe only safe the index of the unfinished block, not the start and end if that is more efficient
  let mut unfinished_start = 0;
  let mut unfininshed_end = 0;
  let mut unfinshed_local = 0;

  workassisting_loop!(loop_arguments, |block_index| {
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
      let local = scan_sequential(&data.input[start .. end], aggregate, &data.output[start .. end]);
      data.temp[block_index as usize].prefix.store(local, Ordering::Relaxed);
      data.temp[block_index as usize].state.store(STATE_PREFIX_AVAILABLE, Ordering::Release);
    } else {
      sequential = false;
      let local = fold_sequential(&data.input[start .. end]);
      // Share own local value
      data.temp[block_index as usize].aggregate.store(local, Ordering::Relaxed);
      data.temp[block_index as usize].state.store(STATE_AGGREGATE_AVAILABLE, Ordering::Release);

      // Check if it has an unfinished block, should only be here if not sequential
      match unfinished_index {
        Some(u_index) => {
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
        data.temp[u_index as usize].prefix.store(aggregate + unfinshed_local, Ordering::Relaxed);
        data.temp[u_index as usize].state.store(STATE_PREFIX_AVAILABLE, Ordering::Release);
  
        // Scan unfinished block
        scan_sequential(&data.input[unfinished_start .. unfininshed_end], aggregate, &data.output[unfinished_start .. unfininshed_end]);
        }
        None => {}
      }
      
         // Replace unfinished block with current block
         unfinished_index = Some(block_index);

         unfinished_start = start;
         unfininshed_end = end;
         unfinshed_local = local;
    }
  });
}

fn finish(workers: &Workers, task: *mut TaskObject<Data>) {
  let _ = unsafe { TaskObject::take_data(task) };
  workers.finish();
}
