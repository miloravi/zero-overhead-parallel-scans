mod core;
mod cases;
mod utils;

fn main() {
  let cpp_enabled = true;

  affinity::set_thread_affinity([0, 4, 8, 12, 2, 6, 10, 14]).unwrap();
  cases::compact::run(cpp_enabled);
  cases::scan::run(cpp_enabled);
}
