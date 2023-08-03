// First use performance cores without hyperthreading,
// then efficiency cores,
// then hyperthreaded performance cores.
pub const AFFINITY_MAPPING: [usize; 24] = [0, 2, 4, 6, 8, 10, 12, 14, 16, 17, 18, 19, 20, 21, 22, 23, 1, 3, 5, 7, 9, 11, 13, 15];
