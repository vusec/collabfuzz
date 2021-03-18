#include "Tracer.h"

#include <fstream>

void Tracer::traceEdge(std::uint64_t source, std::uint64_t target) {
  if (output_path_.empty()) {
    return;
  }

  edge_map_[Edge(source, target)]++;
}

void Tracer::writeData() {
  if (output_path_.empty()) {
    return;
  }

  std::ofstream output_stream(output_path_);
  output_stream << "source,target,count\n";
  for (const auto& [edge, count] : edge_map_) {
    output_stream << std::hex << std::showbase;
    output_stream << edge.getSource() << "," << edge.getTarget() << ",";
    output_stream << std::dec << std::noshowbase;

    output_stream << count << "\n";
  }
}
