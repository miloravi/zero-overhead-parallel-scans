// Uncomment for the CPU you are using

//// AMD Ryzen Threadripper 2950X
pub const AFFINITY_MAPPING: [usize; 32] = [0, 4, 8, 12, 2, 6, 10, 14, 1, 3, 5, 7, 9, 11, 13, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31];
pub const THREAD_COUNTS: [usize; 14] = [1, 2, 3, 4, 6, 8, 10, 12, 14, 16, 20, 24, 28, 32];

//// Intel 12900
// pub const AFFINITY_MAPPING: [usize; 24] = [0, 2, 4, 6, 8, 10, 12, 14, 16, 17, 18, 19, 20, 21, 22, 23, 1, 3, 5, 7, 9, 11, 13, 15];
// pub const THREAD_COUNTS: [usize; 10] = [1, 2, 4, 6, 8, 10, 12, 16, 20, 24];

// Intel i7 - 10750H
// pub const AFFINITY_MAPPING: [usize; 12] = [0, 2, 4, 6, 8, 10, 1, 3, 5, 7, 9, 11];
// pub const THREAD_COUNTS: [usize; 8] = [1, 2, 3, 4, 6, 8, 10, 12];

// Customizable variables:
// Generic max threads and speed up
pub const MAX_THREADS: u32 = 32;
pub const MAX_SPEEDUP: u32 = 6;

// compact benchmark max threads and speed up
pub const COMP_MAX_THREADS: u32 = 32;
pub const COMP_MAX_SPEEDUP: u32 = 6;



