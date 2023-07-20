#include <chrono>
#include <oneapi/tbb.h>

#define RUNS 50

uint randomize(uint64_t seed) {
  seed ^= seed << 13;
  seed ^= seed >> 17;
  seed ^= seed << 5;
  return (uint) seed;
}

bool predicate(uint64_t mask, uint64_t value) {
  value ^= value >> 11;
  value ^= value << 7;
  value ^= value >> 5;
  return (value & mask) == mask;
}

void test_sequential_scan(int size, uint64_t* input, uint64_t* output) {
  int accum = 0;
  for (int i = 0; i < size; i++) {
    accum += input[i];
    output[i] = accum;
  }
}

void test_parallel_scan(int size, uint64_t* input, uint64_t* output) {
  oneapi::tbb::parallel_scan(
    oneapi::tbb::blocked_range<int>(0, size, 1024),
    0,
    [input, output](const oneapi::tbb::blocked_range<int>& r, int sum, bool is_final_scan) -> int {
      int temp = sum;
      int rend = r.end();
      for (int i=r.begin(); i<rend; ++i) {
        temp = temp + input[i];
        if (is_final_scan) {
          output[i] = temp;
        }
      }

      return temp;
    },
    []( int left, int right ) {
      return left + right;
    }
  );
}

void test_sequential_compact(uint64_t mask, int size, uint64_t* input, uint64_t* output) {
  int output_index = 0;
  for (int i = 0; i < size; i++) {
    uint64_t value = input[i];
    if (predicate(mask, value)) {
      output[output_index] = value;
      output_index++;
    }
  }
}

void test_parallel_compact(uint64_t mask, int size, uint64_t* input, uint64_t* output) {
  oneapi::tbb::parallel_scan(
    oneapi::tbb::blocked_range<int>(0, size, 1024),
    0,
    [mask, input, output](const oneapi::tbb::blocked_range<int>& r, int sum, bool is_final_scan) -> int {
      int temp = sum;
      int rend = r.end();
      for (int i=r.begin(); i<rend; ++i) {
        uint64_t value = input[i];
        if (predicate(mask, value)) {
          temp++;
          if (is_final_scan) {
            output[i] = temp;
          }
        }
      }

      return temp;
    },
    []( int left, int right ) {
      return left + right;
    }
  );
}

int main(int argc, char *argv[]) {
  if (argc < 3) {
    printf("Usage: ./main test-case input-size (thread-count)\n");
    return 0;
  }

  // Parse input size
  int size = std::stoi(argv[2]);
  if (size <= 0) {
    printf("input-size should be positive");
    return 0;
  }

  // Allocate input and output arrays
  uint64_t* input = new uint64_t[size];
  uint64_t* output = new uint64_t[size];

  // Initialise input
  for (int i = 0; i < size; i++) {
    input[i] = randomize(i);
  }

  // Switch on test case
  if (std::strcmp(argv[1], "scan-sequential") == 0) {
    // Warm-up run
    test_sequential_scan(size, input, output);

    // Initialise timer
    auto before = std::chrono::high_resolution_clock::now();

    // Perform several runs
    for (int j = 0; j < RUNS; j++) {
      test_sequential_scan(size, input, output);
    }

    // Compute and print average time
    auto msec = std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::high_resolution_clock::now() - before);
    printf("%ld\n", msec.count() / RUNS);

  } else if (std::strcmp(argv[1], "scan-tbb") == 0) {
    // Configure TBB with maximal thread count
    int thread_count = std::stoi(argv[3]);
    if (thread_count <= 0) {
      printf("thread count should be positive");
      return 0;
    }
    tbb::global_control global_limit = tbb::global_control(tbb::global_control::max_allowed_parallelism, thread_count);

    // Warm-up run
    test_parallel_scan(size, input, output);

    // Initialise timer
    auto before = std::chrono::high_resolution_clock::now();

    // Perform several runs
    for (int j = 0; j < RUNS; j++) {
      test_parallel_scan(size, input, output);
    }

    // Compute and print average time
    auto msec = std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::high_resolution_clock::now() - before);
    printf("%ld\n", msec.count() / RUNS);

  } else if (std::strcmp(argv[1], "compact-2-sequential") == 0 || std::strcmp(argv[1], "compact-8-sequential") == 0) {
    int ratio = std::strcmp(argv[1], "compact-2-sequential") == 0 ? 2 : 8;
    int mask = ratio - 1;

    // Warm-up run
    test_sequential_compact(mask, size, input, output);

    // Initialise timer
    auto before = std::chrono::high_resolution_clock::now();

    // Perform several runs
    for (int j = 0; j < RUNS; j++) {
      test_sequential_compact(mask, size, input, output);
    }

    // Compute and print average time
    auto msec = std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::high_resolution_clock::now() - before);
    printf("%ld\n", msec.count() / RUNS);

  } else if (std::strcmp(argv[1], "compact-2-tbb") == 0 || std::strcmp(argv[1], "compact-8-tbb") == 0) {
    int ratio = std::strcmp(argv[1], "compact-2-tbb") == 0 ? 2 : 8;
    int mask = ratio - 1;

    // Configure TBB with maximal thread count
    int thread_count = std::stoi(argv[3]);
    if (thread_count <= 0) {
      printf("thread count should be positive");
      return 0;
    }
    tbb::global_control global_limit = tbb::global_control(tbb::global_control::max_allowed_parallelism, thread_count);

    // Warm-up run
    test_parallel_compact(mask, size, input, output);

    // Initialise timer
    auto before = std::chrono::high_resolution_clock::now();

    // Perform several runs
    for (int j = 0; j < RUNS; j++) {
      test_parallel_compact(mask, size, input, output);
    }

    // Compute and print average time
    auto msec = std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::high_resolution_clock::now() - before);
    printf("%ld\n", msec.count() / RUNS);

  } else {
    printf("Unknown test case.");
  }

  return 0;
}
