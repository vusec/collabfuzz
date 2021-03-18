#include "Tracer.h"

#include <fstream>
#include <gsl/gsl>

void Tracer::traceCondition(std::uint64_t instruction_id,
                            std::uint64_t total_cases,
                            std::uint64_t current_case) {
  if (output_path_.empty()) {
    return;
  }

  auto [iter, _] = condition_map_.try_emplace(instruction_id, total_cases);

  // Since try_emplace does not necessarily create an entry, check that
  // total_cases matches whatever is in the map.
  Expects(iter->second.size() == total_cases);

  iter->second[current_case] = true;
}

void Tracer::writeData() {
  if (output_path_.empty()) {
    return;
  }

  std::ofstream output_stream(output_path_);
  output_stream << "condition_id,cases\n";
  for (const auto& [condition_id, cases] : condition_map_) {
    output_stream << std::hex << std::showbase;
    output_stream << condition_id << ",";
    output_stream << std::dec << std::noshowbase;

    for (auto cond_case : cases) {
      if (cond_case) {
        output_stream << "1";
      } else {
        output_stream << "0";
      }
    }

    output_stream << '\n';
  }
}
