#include "collabfuzz/IDAssigner.h"

#include "llvm/ADT/Statistic.h"
#include "llvm/IR/CFG.h"
#include "llvm/IR/Constants.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LegacyPassManager.h"
#include "llvm/Pass.h"
#include "llvm/Support/Debug.h"
#include "llvm/Transforms/IPO/PassManagerBuilder.h"
#include "llvm/Transforms/Utils.h"
#include "llvm/Transforms/Utils/BasicBlockUtils.h"
#include "llvm/Transforms/Utils/ModuleUtils.h"

#include <iterator>

#define DEBUG_TYPE "edge-tracer"

using namespace llvm;
using collabfuzz::IDAssigner;

STATISTIC(injectedCalls, "Number of injected calls");

namespace {
class EdgeTracer : public ModulePass {
  const char *const CtorName = "__edge_tracer_create";
  const char *const DtorName = "__edge_tracer_destroy";
  const char *const CallbackName = "__edge_tracer_trace";

  IntegerType *IdentifierTy;
  FunctionType *CtorDtorTy;
  FunctionType *CallbackTy;

  FunctionCallee Callback;
  const IDAssigner::IdentifiersMap *IdMap;

  void addDeclarations(Module &M);
  void instrumentFunction(Function &F);
  void emitEdgeInstrumentation(IRBuilder<> &IRB, BasicBlock &Source,
                               BasicBlock &Target);

public:
  static char ID;

  EdgeTracer() : ModulePass(ID) {}

  void getAnalysisUsage(AnalysisUsage &AU) const override {
    AU.addRequired<collabfuzz::IDAssigner>();
  }

  bool doInitialization(Module &M) override;
  bool runOnModule(Module &M) override;
};
} // namespace

char EdgeTracer::ID = 0;

bool EdgeTracer::doInitialization(Module &M) {
  auto &C = M.getContext();
  auto VoidTy = Type::getVoidTy(C);

  IdentifierTy = Type::getInt64Ty(C);
  CtorDtorTy = FunctionType::get(VoidTy, false);
  CallbackTy = FunctionType::get(VoidTy, {IdentifierTy, IdentifierTy}, false);

  return true;
}

void EdgeTracer::addDeclarations(Module &M) {
  LLVM_DEBUG(dbgs() << "Emitting declarations.\n");

  auto Ctor = M.getOrInsertFunction(CtorName, CtorDtorTy);
  appendToGlobalCtors(M, cast<Function>(Ctor.getCallee()), 0);

  auto Dtor = M.getOrInsertFunction(DtorName, CtorDtorTy);
  appendToGlobalDtors(M, cast<Function>(Dtor.getCallee()), 0);

  Callback = M.getOrInsertFunction(CallbackName, CallbackTy);
}

void EdgeTracer::emitEdgeInstrumentation(IRBuilder<> &IRB, BasicBlock &Source,
                                         BasicBlock &Target) {
  auto SourceID = IdMap->lookup(&Source);
  assert(SourceID != 0);
  auto SourceIDValue = ConstantInt::get(IdentifierTy, SourceID);

  auto TargetID = IdMap->lookup(&Target);
  assert(TargetID != 0);
  auto TargetIDValue = ConstantInt::get(IdentifierTy, TargetID);

  IRB.CreateCall(Callback, {SourceIDValue, TargetIDValue});

  LLVM_DEBUG(dbgs() << "      Emitting call to support library.\n");
  ++injectedCalls;
}

void EdgeTracer::instrumentFunction(Function &F) {
  LLVM_DEBUG(dbgs() << "Instrumenting function: " << F.getName() << '\n');

  SmallVector<BasicBlock *, 16> OriginalBlocks;
  for (auto &BB : F) {
    OriginalBlocks.push_back(&BB);
  }

  for (auto *CurrentBB : OriginalBlocks) {
    LLVM_DEBUG(dbgs() << "  Block: " << IdMap->lookup(CurrentBB) << '\n');

    auto NumPredecessors =
        std::distance(pred_begin(CurrentBB), pred_end(CurrentBB));
    auto NumSuccessors =
        std::distance(succ_begin(CurrentBB), succ_end(CurrentBB));

    if (NumPredecessors == 1) {
      LLVM_DEBUG(dbgs() << "    Block has one predecessor.\n");

      IRBuilder<> IRB(CurrentBB, CurrentBB->getFirstInsertionPt());
      auto *PredBB = *pred_begin(CurrentBB);
      emitEdgeInstrumentation(IRB, *PredBB, *CurrentBB);
    }

    if (NumSuccessors == 1) {
      LLVM_DEBUG(dbgs() << "    Block has one successor.\n");

      IRBuilder<> IRB(CurrentBB->getTerminator());
      auto *SuccBB = *succ_begin(CurrentBB);
      emitEdgeInstrumentation(IRB, *CurrentBB, *SuccBB);
    }

    // This is the same condition that SplitAllCriticalEdges uses to select
    // possible critical edges candidates.
    if (NumSuccessors > 1 && !isa<IndirectBrInst>(CurrentBB->getTerminator())) {
      for (auto Iter = succ_begin(CurrentBB), End = succ_end(CurrentBB);
           Iter != End; ++Iter) {
        if (BasicBlock *CritBlock = SplitCriticalEdge(CurrentBB, Iter)) {
          LLVM_DEBUG(dbgs() << "    Critical edge found.\n");

          // Only original blocks should be traced, thus the critical edge
          // contains a callback that connects its only parent with its only
          // child.
          IRBuilder<> IRB(CritBlock, CritBlock->getFirstInsertionPt());
          emitEdgeInstrumentation(IRB, *CurrentBB, **succ_begin(CritBlock));
        }
      }
    }
  }
}

bool EdgeTracer::runOnModule(Module &M) {
  addDeclarations(M);
  IdMap = &getAnalysis<IDAssigner>().getIdentifiersMap();

  for (auto &F : M) {
    auto FuncName = F.getName();
    if (FuncName != CtorName && FuncName != DtorName &&
        FuncName != CallbackName) {
      instrumentFunction(F);
    }
  }

  return true;
}

static RegisterPass<EdgeTracer>
    X1("edge-tracer", "Insert instrumentation for edge tracing", false, false);

static void registerEdgeTracerPass(const PassManagerBuilder &,
                                   legacy::PassManagerBase &PM) {
  PM.add(new EdgeTracer());
}

static RegisterStandardPasses X2(PassManagerBuilder::EP_OptimizerLast,
                                 registerEdgeTracerPass);

static RegisterStandardPasses X3(PassManagerBuilder::EP_EnabledOnOptLevel0,
                                 registerEdgeTracerPass);
