use core::sync::atomic::{Ordering, AtomicU64, AtomicUsize};
use crate::cases::compact::{compact_sequential, count_sequential};
use crate::core::worker::*;
use crate::core::task::*;
use crate::core::workassisting_loop::*;
use crate::cases::compact::chained::{ Data, BlockInfo, reset, BLOCK_SIZE, STATE_AGGREGATE_AVAILABLE, STATE_PREFIX_AVAILABLE, STATE_INITIALIZED };

pub fn create_temp(size: usize) -> Box<[BlockInfo]> {
  (0 .. (size as u64 + BLOCK_SIZE - 1) / BLOCK_SIZE).map(|_| BlockInfo{
    state: AtomicU64::new(STATE_INITIALIZED), aggregate: AtomicUsize::new(0), prefix: AtomicUsize::new(0)
  }).collect()
}

pub fn create_task(mask: u64, input: &[u64], temp: &[BlockInfo], output: &[AtomicU64], output_count: &AtomicUsize) -> Task {
  reset(temp);
  Task::new_dataparallel::<Data>(run, finish, Data{ mask, input, temp, output, output_count }, ((input.len() as u64 + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32, false)
}

fn run(_workers: &Workers, task: *const TaskObject<Data>, loop_arguments: LoopArguments) {
  let data = unsafe { TaskObject::get_data(task) };
  workassisting_loop!(loop_arguments, |block_index| {
    // Local scan
    // reduce-then-scan
    let start = block_index as usize * BLOCK_SIZE as usize;
    let end = ((block_index as usize + 1) * BLOCK_SIZE as usize).min(data.input.len());

    if block_index == 0 {
      let local = compact_sequential(data.mask, &data.input[start .. end], data.output, 0);
      data.temp[block_index as usize].prefix.store(local, Ordering::Relaxed);
      data.temp[block_index as usize].state.store(STATE_PREFIX_AVAILABLE, Ordering::Release);
    } else {
      let local = count_sequential(data.mask, &data.input[start .. end]);
      // Share own local value
      data.temp[block_index as usize].aggregate.store(local, Ordering::Relaxed);
      data.temp[block_index as usize].state.store(STATE_AGGREGATE_AVAILABLE, Ordering::Release);

      // Find aggregate
      let mut aggregate = 0;
      let mut previous = block_index - 1;

      loop {
        let previous_state = data.temp[previous as usize].state.load(Ordering::Acquire);
        if previous_state == STATE_PREFIX_AVAILABLE {
          aggregate = data.temp[previous as usize].prefix.load(Ordering::Acquire) + aggregate;
          break;
        } else {
          // Continue looping until the state of previous block changes.
        }
      }

      // Make aggregate available
      data.temp[block_index as usize].prefix.store(aggregate + local, Ordering::Relaxed);
      data.temp[block_index as usize].state.store(STATE_PREFIX_AVAILABLE, Ordering::Release);
      compact_sequential(data.mask, &data.input[start .. end], data.output, aggregate as usize);
    }
  });
}

fn finish(workers: &Workers, task: *mut TaskObject<Data>) {
  let data = unsafe { TaskObject::take_data(task) };
  let count = data.temp[data.temp.len() - 1].prefix.load(Ordering::Relaxed);
  data.output_count.store(count, Ordering::Relaxed);
  workers.finish();
}
