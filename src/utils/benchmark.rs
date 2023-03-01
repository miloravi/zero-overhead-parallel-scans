use core::fmt::Debug;
use std::time;
use std::fs::File;
use std::io::{prelude::*, BufWriter};

pub struct Benchmarker<T> {
  chart_style: ChartStyle,
  name: String,
  max_speedup: u32,
  max_threads: u32,
  reference_time: u64,
  expected: T,
  output: Vec<(String, u32, Option<u32>, bool, Vec<f32>)>
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum ChartStyle {
  Left,
  LeftWithKey,
  Right
}

pub fn benchmark<T: Debug + Eq, Ref: FnMut() -> T>(chart_style: ChartStyle, name: &str, reference: Ref) -> Benchmarker<T> {
  benchmark_with_max_speedup(chart_style, name, reference, 16, 8)
}

pub fn benchmark_with_max_speedup<T: Debug + Eq, Ref: FnMut() -> T>(chart_style: ChartStyle, name: &str, reference: Ref, max_threads: u32, max_speedup: u32) -> Benchmarker<T> {
  println!("");
  println!("Benchmark {}", name);
  let (expected, reference_time) = time(50, reference);
  println!("Sequential   {} ms", reference_time / 1000);
  Benchmarker{ chart_style, name: name.to_owned(), max_threads, max_speedup, reference_time, expected, output: vec![] }
}

impl<T: Copy + Debug + Eq + Send> Benchmarker<T> {
  pub fn parallel<Par: FnMut(usize) -> T>(mut self, name: &str, chart_line_style: u32, point_type: Option<u32>, our: bool, mut parallel: Par) -> Self {
    println!("{}", name);
    let mut results = vec![];
    for thread_count in [1, 2, 3, 4, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32] {
      if thread_count > self.max_threads as usize {
        break;
      }

      let (value, time) = time(50, || parallel(thread_count));
      assert_eq!(self.expected, value);
      let relative = self.reference_time as f32 / time as f32;
      results.push(relative);
      println!("  {:02} threads {} ms ({:.2}x)", thread_count, time / 1000, relative);
    }
    self.output.push((name.to_owned(), chart_line_style, point_type, our, results));
    self
  }
}

impl<T> Drop for Benchmarker<T> {
  fn drop(&mut self) {
    std::fs::create_dir_all("./results").unwrap();
    let filename = "./results/".to_owned() + &self.name.replace(' ', "_").replace('(', "").replace(')', "").replace('=', "_").replace('/', "_");

    let file_gnuplot = File::create(filename.clone() + ".gnuplot").unwrap();
    let mut gnuplot = BufWriter::new(&file_gnuplot);
    writeln!(&mut gnuplot, "set title \"{}\"", self.name).unwrap();
    writeln!(&mut gnuplot, "set terminal pdf size {},2.6", if self.chart_style == ChartStyle::Right {2.3} else {2.6}).unwrap();
    writeln!(&mut gnuplot, "set output \"{}\"", filename.clone() + ".pdf").unwrap();
    if self.chart_style == ChartStyle::LeftWithKey {
      writeln!(&mut gnuplot, "set key on").unwrap();
      writeln!(&mut gnuplot, "set key top left Left reverse").unwrap();
    } else {
      writeln!(&mut gnuplot, "set key off").unwrap();
    }
    writeln!(&mut gnuplot, "set xrange [1:{}]", self.max_threads).unwrap();
    writeln!(&mut gnuplot, "set xtics (1, 4, 8, 12, 16, 20, 24, 28, 32)").unwrap();
    writeln!(&mut gnuplot, "set xlabel \"Threads\"").unwrap();
    writeln!(&mut gnuplot, "set yrange [0:{}]", self.max_speedup).unwrap();
    if self.chart_style == ChartStyle::Right {
      writeln!(&mut gnuplot, "set format y \"\"").unwrap();
    } else {
      writeln!(&mut gnuplot, "set ylabel \"Speedup\"").unwrap();
    }

    write!(&mut gnuplot, "plot ").unwrap();
    for (idx, result) in self.output.iter().enumerate() {
      if idx != 0 {
        write!(&mut gnuplot, ", \\\n  ").unwrap();
      }
      write!(&mut gnuplot, "'{}.dat' using 1:{} title \"{}\" ls {} lw 1 pointsize {}", filename, idx+2, result.0, result.1, if result.3 { 0.7 } else { 0.6 }).unwrap();
      if let Some(point_type) = result.2 {
        write!(&mut gnuplot, " pointtype {}", point_type).unwrap();
      }
      write!(&mut gnuplot, " with linespoints").unwrap();
    }
    writeln!(&mut gnuplot, "").unwrap();

    let file_data = File::create(filename.clone() + ".dat").unwrap();
    let mut writer = BufWriter::new(&file_data);
    write!(&mut writer, "# Benchmark {}\n", self.name).unwrap();
    write!(&mut writer, "# Speedup compared to a sequential implementation.\n").unwrap();
    
    // Header
    write!(&mut writer, "# NCPU").unwrap();
    for result in &self.output {
      write!(&mut writer, "\t{}", result.0).unwrap();
    }
    write!(&mut writer, "\n").unwrap();

    for (idx, thread_count) in [1, 2, 3, 4, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32].iter().enumerate() {
      write!(&mut writer, "{}", thread_count).unwrap();
      for result in &self.output {
        if idx < result.4.len() {
          write!(&mut writer, "\t{}", result.4[idx]).unwrap();
        } else {
          write!(&mut writer, "\t").unwrap();
        }
      }
      write!(&mut writer, "\n").unwrap();
    }

    std::process::Command::new("gnuplot")
      .arg(filename + ".gnuplot")
      .spawn()
      .expect("gnuplot failed");
  }
}

pub fn time<T: Debug + Eq, F: FnMut() -> T>(runs: usize, mut f: F) -> (T, u64) {
  let first = f();
  
  let timer = time::Instant::now();
  for _ in 0 .. runs {
    assert_eq!(first, f());
  }

  (first, (timer.elapsed().as_micros() / runs as u128) as u64)
}
