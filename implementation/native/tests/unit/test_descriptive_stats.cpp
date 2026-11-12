#include <cassert>
#include <cmath>
#include <iostream>
#include <vector>
#include "apsinsight/numerics/descriptive_stats.hpp"

int main() {
  using namespace apsinsight::numerics;

  std::vector<double> data = {1.0, 2.0, 3.0, 4.0};
  auto stats = descriptive_stats(data);

  assert(stats.count == 4);
  assert(std::abs(stats.mean - 2.5) < 1e-9);
  assert(stats.min == 1.0);
  assert(stats.max == 4.0);
  // sample variance of 1,2,3,4 is 1.666...
  assert(std::abs(stats.variance - 1.6666666666666667) < 1e-9);

  std::cout << "C++ descriptive_stats test passed\n";
  return 0;
}
