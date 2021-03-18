#include <fstream>
#include <iomanip>
#include <vector>
#include "llvm/Pass.h"
#include "llvm/Analysis/CallGraphSCCPass.h"
#include "llvm/Analysis/CallGraph.h"
#include "llvm/IR/Dominators.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/InlineAsm.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/CFG.h"
#include "llvm/ADT/SCCIterator.h"
#include "llvm/ADT/PostOrderIterator.h"
#include "llvm/ADT/BreadthFirstIterator.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Support/MD5.h"
#include "llvm/Support/CommandLine.h"

#include "collabfuzz/IDAssigner.h"

#include <nlohmann/json.hpp>
using json = nlohmann::json;

using namespace llvm;
using collabfuzz::IDAssigner;

class BBReachableGraph;

uint64_t getBBID(BasicBlock &BB);

uint64_t getBBID(BasicBlock &BB) {
  std::string name = BB.getName();
  if (name.empty()) {
    errs() << "bb with no name\n";
  }
  return MD5Hash(name);
}

namespace {
  struct BBIDs : public ModulePass {
    static char ID;
    BBIDs() : ModulePass(ID) {}

    bool runOnModule(Module &M) override {
      // Scan through the module and assign a ID consisting of the parent function name
      // and a unique, positive (i.e., non-zero) ID to every basic block.
      unsigned total_count = 0;
      for (Module::iterator MI = M.begin(), ME = M.end(); MI != ME; ++MI) {
        for (Function::iterator BB = MI->begin(), BE = MI->end(); BB != BE; ++BB) {
          BasicBlock& curr = *BB;
          curr.setName(std::to_string(++total_count));
        }
      }
      errs() << "Total Number of Basic Blocks: " << total_count << "\n";

      return true;
    }
  };
}

char BBIDs::ID = 0;
static RegisterPass<BBIDs> Y("bbids", "Hello World Pass",
                             false /* Only looks at CFG */,
                             false /* Analysis Pass */);

namespace {
  struct InstrumentBBIDs : public BasicBlockPass {
    static char ID;
    InstrumentBBIDs() : BasicBlockPass(ID) {}

    std::string intToHex(std::string integer) {
      std::stringstream stream;
      stream << std::hex << std::stoi(integer);
      return stream.str();
    }

    void insertNopImmediate(BasicBlock &BB, std::string immediate) {
      std::vector<llvm::Value*> AsmArgs;
      std::vector<llvm::Type *> AsmArgTypes;
      llvm::FunctionType *AsmFTy =
        llvm::FunctionType::get(Type::getVoidTy(BB.getContext()), false);
      llvm::InlineAsm *IA = llvm::InlineAsm::get(AsmFTy,
                                                 /* AsmString */ "nopl 0x" + immediate + "(%eax)",
                                                 /* Constraints */ "",
                                                 /* hasSideEffects */ false,
                                                 /* IsAlignStack */ false,
                                                 llvm::InlineAsm::AD_ATT);
      CallInst::Create(IA, AsmArgs, "", BB.getFirstNonPHIOrDbgOrLifetime());
    }

    bool runOnBasicBlock(BasicBlock &BB) override {
      // TODO set volatile?
      insertNopImmediate(BB, intToHex(BB.getName()));
      return true;
    }
  };
}

char InstrumentBBIDs::ID = 1;
static RegisterPass<InstrumentBBIDs> X("inst-bbids", "Instrument BBIDs Pass",
                             false /* Only looks at CFG */,
                             false /* Analysis Pass */);

class BBReachableNode {
public:
  using ReachableNodesTy = std::set<const BBReachableNode *>;
  using const_iterator = ReachableNodesTy::const_iterator;

  const BasicBlock* BB;

  explicit BBReachableNode(const BasicBlock* BasicBlock) : BB(BasicBlock) {}
  ~BBReachableNode() {}

  void addReachable(const BBReachableNode* reachable_node) {
    ReachableNodes.insert(reachable_node);
  }

  inline const_iterator begin() const { return ReachableNodes.begin(); }
  inline const_iterator end() const { return ReachableNodes.end(); }
  inline bool empty() const { return ReachableNodes.empty(); }
  inline unsigned size() const { return static_cast<unsigned>(ReachableNodes.size()); }

private:
  ReachableNodesTy ReachableNodes;
};

class BBReachableGraph {
  using BBMapTy = std::map<const BasicBlock *, std::unique_ptr<BBReachableNode>>;
  BBMapTy BBMap;
  const Module& M;
  std::unordered_map<const FunctionType *, std::set<const Function *>> function_type_to_functions;
public:
  using const_iterator = BBMapTy::const_iterator;
  BBReachableNode EntryNode;

  inline const_iterator begin() const { return BBMap.begin(); }
  inline const_iterator end() const { return BBMap.end(); }
  inline unsigned long size() const { return BBMap.size(); }


  explicit BBReachableGraph(const Module& Module) :
    M(Module), EntryNode(nullptr) /*, CG(Module)*/ {

    collectIndirectFunctionTargets();

    for (const auto& func : M) {

      // add entry node to point to all possible entries to the module, same as CallGraph:
      // If this function has external linkage or has its address taken, anything
      // could call it.
      if (!func.hasLocalLinkage() || func.hasAddressTaken()) {
        addACalledFunction(&EntryNode, &func);
      }

      for (const auto& bb : func) {
        addToBBReachableGraph(&bb);
      }
    }
  }

  ~BBReachableGraph() {}

  void addToBBReachableGraph(const BasicBlock* BB) {
    BBReachableNode* BBRN = getOrInsertBB(BB);
    for (succ_const_iterator SI = succ_begin(BB), E = succ_end(BB); SI != E; ++SI) {
      const BasicBlock* sbb = *SI;
      BBRN->addReachable(getOrInsertBB(sbb));
    }
    addCalledFunctions(BBRN, BB);
  }

  BBReachableNode* getOrInsertBB(const BasicBlock *BB) {
    auto &BBN = BBMap[BB];
    if (BBN) return BBN.get();
    BBN = llvm::make_unique<BBReachableNode>(const_cast<BasicBlock *>(BB));
    return BBN.get();
  }

private:
  void collectIndirectFunctionTargets() {
    for (auto F = M.begin(); F != M.end(); ++F) {
      const Function *f = &*F;
      if (f->isDeclaration()) continue;  // ignore external functions as they don't have basic blocks
      const FunctionType* ft = F->getFunctionType();
      auto search = function_type_to_functions.find(ft);
      if (search != function_type_to_functions.end()) {
        search->second.insert(f);
      } else {
        std::set<const Function *> matching_functions;
        matching_functions.insert(f);
        function_type_to_functions.insert_or_assign(ft, matching_functions);
      }
    }
  }

  void addCalledFunctions(BBReachableNode* caller, const BasicBlock* bb) {
    for (const Instruction& I : *bb) {
      const CallInst* inst = dyn_cast<CallInst>(&I);
      if (inst) {
        if (inst->isInlineAsm()) continue;
        const Function* called = inst->getCalledFunction();
        const FunctionType* func_type = inst->getFunctionType();
        if (inst->isIndirectCall() || !called) {
          addIndirectlyCalledFunctions(caller, func_type);
        } else {
          addACalledFunction(caller, called);
        }
      }
    }
  }

  void addIndirectlyCalledFunctions(BBReachableNode* caller, const FunctionType* func_type) {
    auto search = function_type_to_functions.find(func_type);
    if (search != function_type_to_functions.end()) {
      for (const Function* called_func : search->second) {
        addACalledFunction(caller, called_func);
      }
    }
  }

  void addACalledFunction(BBReachableNode* caller, const Function* called_func) {
    if (called_func->isDeclaration()) return;  // has no basic blocks
    const BasicBlock* called_bb = &called_func->getEntryBlock();
    caller->addReachable(getOrInsertBB(called_bb));
  }
};

template <>
struct llvm::GraphTraits<const BBReachableNode *> {

  using NodeRef = const BBReachableNode *;
  using ChildIteratorType = BBReachableNode::const_iterator;

  static NodeRef getEntryNode(const BBReachableNode *BBRN) {
    return BBRN;
  }

  static ChildIteratorType child_begin(const NodeRef N) {
    return ChildIteratorType(N->begin());
  }

  static ChildIteratorType child_end(const NodeRef N) {
    return ChildIteratorType(N->end());
  }
};

template <>
struct llvm::GraphTraits<const BBReachableGraph *> : public GraphTraits<const BBReachableNode *> {
  // implement some accessors to allow graph algorithms on the wrapper class

  // the entry node does not contain a BasicBlock
  static NodeRef getEntryNode(const BBReachableGraph *BBRG) {
    return &BBRG->EntryNode;
  }

  using nodes_iterator = BBReachableGraph::const_iterator;

  static nodes_iterator nodes_begin(const BBReachableGraph *BBRG) {
    return nodes_iterator(BBRG->begin());
  }

  static nodes_iterator nodes_end(const BBReachableGraph *BBRG) {
    return nodes_iterator(BBRG->end());
  }
};

// Add a pass argument
static cl::opt<std::string> BBOutFile("bb-reach-output",
                                      cl::desc("Static analysis output file"));

namespace {
  struct BBReachablePass : public ModulePass {
    static char ID;
    BBReachablePass() : ModulePass(ID) {}

    void getAnalysisUsage(AnalysisUsage &AU) const override {
      AU.addRequired<collabfuzz::IDAssigner>();
    }

    bool runOnModule(Module &M) override {
      auto IdMap = &getAnalysis<IDAssigner>().getIdentifiersMap();
      const BBReachableGraph BBR(M);
      std::map<long, std::vector<long>> adjacency_list;

      unsigned long num_bbs = BBR.size();
      unsigned long ctr = 0;
      for (const auto& BBRN : BBR) {
        outs() << "BB: " << ctr++ << "/" << num_bbs << "\r";
        const BBReachableNode* node = BBRN.second.get();
        auto node_id = IdMap->lookup(node->BB);
        auto cur_children = json::array();
        for (auto BBCI = node->begin(), BBCIE = node->end(); BBCI != BBCIE; ++BBCI) {
          cur_children.push_back(IdMap->lookup((*BBCI)->BB));
        }
        adjacency_list.insert({node_id, cur_children});
      }

      json j(adjacency_list);
      outs() << "\ndone, writing output file: " << BBOutFile << "\n";
      std::ofstream o(BBOutFile);
      o << std::setw(4) << j << std::endl;
      return false;
    }
  };
}

char BBReachablePass::ID = 2;
static RegisterPass<BBReachablePass> Z("bb-reach", "Basic Block Reachable Pass",
                             false /* CFGOnly */,
                             false /* is_analysis */);
