#pragma once

#include <vector>
#include <cmath>
#include <stdexcept>

namespace apsinsight::numerics {

struct DescriptiveStats {
  std::size_t count = 0;
  double mean = 0.0;
  double variance = 0.0;
  double min = 0.0;
  double max = 0.0;
};

/// Compute descriptive statistics for a vector of doubles.
/// Throws on empty input.
DescriptiveStats descriptive_stats(const std::vector<double>& values);

} // namespace apsinsight::numerics
