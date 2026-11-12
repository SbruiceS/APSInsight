#include "apsinsight/numerics/descriptive_stats.hpp"

namespace apsinsight::numerics {

DescriptiveStats descriptive_stats(const std::vector<double>& values) {
  if (values.empty()) {
    throw std::invalid_argument("Cannot compute statistics on empty vector");
  }

  DescriptiveStats result;
  result.count = values.size();
  result.min = values[0];
  result.max = values[0];

  double sum = 0.0;
  for (double v : values) {
    sum += v;
    if (v < result.min) result.min = v;
    if (v > result.max) result.max = v;
  }
  result.mean = sum / static_cast<double>(result.count);

  if (result.count > 1) {
    double sq_sum = 0.0;
    for (double v : values) {
      double diff = v - result.mean;
      sq_sum += diff * diff;
    }
    result.variance = sq_sum / static_cast<double>(result.count - 1);
  }

  return result;
}

} // namespace apsinsight::numerics
