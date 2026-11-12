cmake_minimum_required(VERSION 3.25)
project(APSInsightNative VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

add_library(apsinsight_numerics STATIC
  src/numerics/descriptive_stats.cpp
)
target_include_directories(apsinsight_numerics PUBLIC include)

enable_testing()
add_executable(test_descriptive_stats tests/unit/test_descriptive_stats.cpp)
target_link_libraries(test_descriptive_stats PRIVATE apsinsight_numerics)
add_test(NAME descriptive_stats COMMAND test_descriptive_stats)