#include <boost/program_options.hpp>
#include <cstdint>
#include <filesystem>
#include <gsl/gsl>
#include <iostream>
#include <memory>
#include <mutex>

#include "Tracer.h"

namespace po = boost::program_options;
namespace fs = std::filesystem;

static const char* const kEnvPrefix = "TRACER_";
static const char* const kEnableFileLabel = "enable_file_output";
static const char* const kOutputFileLabel = "output_file";

// The order of execution of constructors and destructors cannot be relied
// upon and thus the tracer object needs to be allocated and deallocated
// manually.

struct StaticVars {
  std::ios_base::Init ios_init_;
  std::unique_ptr<Tracer> tracer_;
};

static std::mutex static_vars_mutex;
static gsl::owner<StaticVars*> static_vars = nullptr;

// NOLINTNEXTLINE(readability-identifier-naming)
extern "C" void __edge_tracer_create() {
  const std::lock_guard lock(static_vars_mutex);

  if (static_vars != nullptr) {
    // The constructor may be called multiple times
    return;
  }
  static_vars = new StaticVars();

  // clang-format off
  po::options_description desc;
  desc.add_options()
    (kEnableFileLabel, po::value<bool>()->default_value(false))
    (kOutputFileLabel,
     po::value<std::string>()->default_value("trace_data.csv"));
  // clang-format on

  try {
    po::variables_map vars_map;
    po::store(po::parse_environment(desc, kEnvPrefix), vars_map);
    po::notify(vars_map);

    fs::path output_path;
    if (vars_map[kEnableFileLabel].as<bool>()) {
      output_path = vars_map[kOutputFileLabel].as<std::string>();
    }

    static_vars->tracer_ = std::make_unique<Tracer>(output_path);
  } catch (std::exception& ex) {
    std::cerr << "tracer error: " << ex.what() << std::endl;

    // Local dtor is not run when calling std::exit, so the mutex needs to be
    // released manually. After this, __edge_tracer_destroy will run.
    static_vars_mutex.unlock();
    std::exit(1);
  }
}

// NOLINTNEXTLINE(readability-identifier-naming)
extern "C" void __edge_tracer_trace(std::uint64_t source,
                                    std::uint64_t target) {
  const std::lock_guard lock(static_vars_mutex);

  // An instrumented constructor may be called before __edge_tracer_create is
  // run.
  if (static_vars != nullptr) {
    static_vars->tracer_->traceEdge(source, target);
  }
}

// NOLINTNEXTLINE(readability-identifier-naming)
extern "C" void __edge_tracer_destroy() {
  const std::lock_guard lock(static_vars_mutex);

  if (static_vars == nullptr) {
    // The destructor may be called multiple times
    return;
  }

  // When __edge_tracer_create fails, __edge_tracer_destroy is run anyway.
  if (static_vars->tracer_ != nullptr) {
    static_vars->tracer_->writeData();
    static_vars->tracer_.reset();
  }

  delete static_vars;
  static_vars = nullptr;
}
