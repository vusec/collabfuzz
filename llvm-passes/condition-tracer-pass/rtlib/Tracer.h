#ifndef RTLIB_TRACER_H
#define RTLIB_TRACER_H

#include <cstdint>
#include <filesystem>
#include <unordered_map>
#include <vector>

class Tracer {
  std::filesystem::path output_path_;
  std::unordered_map<std::uint64_t, std::vector<bool>> condition_map_;

 public:
  Tracer(std::filesystem::path& output_path) : output_path_(output_path){};

  void traceCondition(std::uint64_t instruction_id, std::uint64_t total_cases,
                      std::uint64_t current_case);
  void writeData();
};

#endif
