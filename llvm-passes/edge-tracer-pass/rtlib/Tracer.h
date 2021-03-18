#ifndef RTLIB_TRACER_H
#define RTLIB_TRACER_H

#include <cstdint>
#include <filesystem>
#include <unordered_map>

class Edge {
  std::uint64_t source_;
  std::uint64_t target_;

  friend bool operator==(const Edge& lhs, const Edge& rhs) {
    return lhs.source_ == rhs.source_ && lhs.target_ == rhs.target_;
  }

 public:
  Edge(std::uint64_t source, std::uint64_t target)
      : source_(source), target_(target) {}

  [[nodiscard]] std::uint64_t getSource() const noexcept { return source_; }
  [[nodiscard]] std::uint64_t getTarget() const noexcept { return target_; }
};

namespace std {
template <>
struct hash<Edge> {
  std::size_t operator()(const Edge& edge) const noexcept {
    // Given that the identifiers are serial, this should provide a unique
    // hash for reasonably small programs.
    auto source = edge.getSource();

    constexpr auto rotate_amount = sizeof(source) / 2;
    auto rotated_source =
        source << rotate_amount | source >> (sizeof(source) - rotate_amount);

    return rotated_source ^ edge.getTarget();
  }
};
}  // namespace std

class Tracer {
  std::filesystem::path output_path_;
  std::unordered_map<Edge, std::uint64_t> edge_map_;

 public:
  Tracer(std::filesystem::path& output_path) : output_path_(output_path){};

  void traceEdge(std::uint64_t source, std::uint64_t target);
  void writeData();
};

#endif
