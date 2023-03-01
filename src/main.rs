mod core;
mod cases;
mod utils;

fn main() {
  affinity::set_thread_affinity([0, 4, 8, 12, 2, 6, 10, 14]).unwrap();
  cases::compact::run();
  cases::scan::run();
}
