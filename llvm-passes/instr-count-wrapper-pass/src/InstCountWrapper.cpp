#include "collabfuzz/IDAssigner.h"

#include "llvm/ADT/Statistic.h"
#include "llvm/IR/CFG.h"
#include "llvm/IR/Constants.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/InstIterator.h"
#include "llvm/IR/InstVisitor.h"
#include "llvm/IR/LegacyPassManager.h"
#include "llvm/Pass.h"
#include "llvm/Support/Debug.h"
#include "llvm/Transforms/IPO/PassManagerBuilder.h"
#include "llvm/Transforms/Utils.h"
#include "llvm/Transforms/Utils/BasicBlockUtils.h"
#include "llvm/Transforms/Utils/ModuleUtils.h"

#include <iterator>

#define DEBUG_TYPE "inst-count-wrapper"

// Modifying this pass probably requires modifying BBTaintTracer as well

using namespace llvm;
using collabfuzz::IDAssigner;

STATISTIC(injectedCalls, "Number of injected calls");

namespace {
class InstCountWrapper : public ModulePass {
  friend class ConditionVisitor;

  const char *const CtorName = "__inst_count_create";
  const char *const DtorName = "__inst_count_destroy";
  const char *const CallbackName = "__inst_count_trace";

  IntegerType *IdentifierTy;
  IntegerType *TracedValueTy;
  FunctionType *CtorDtorTy;
  FunctionType *CallbackTy;

  FunctionCallee Callback;
  const IDAssigner::IdentifiersMap *IdMap;

  void addDeclarations(Module &M);
  void instrumentFunction(Function &F);
  void emitTerminatorInstrumentation(Instruction &CurrentTerm,
                                     Value &TracedValue) const;

public:
  static char ID;

  InstCountWrapper() : ModulePass(ID) {}

  void getAnalysisUsage(AnalysisUsage &AU) const override {
    AU.setPreservesCFG();
    AU.addRequired<collabfuzz::IDAssigner>();
  }

  bool doInitialization(Module &M) override;
  bool runOnModule(Module &M) override;
};

class ConditionVisitor : public InstVisitor<ConditionVisitor> {
  const InstCountWrapper &Pass;

public:
  ConditionVisitor(const InstCountWrapper &Pass) : Pass(Pass){};

  void visitBranchInst(BranchInst &BranchTerm) {
    LLVM_DEBUG(dbgs() << "    Block has a br terminator.\n");
    assert(BranchTerm.isConditional());

    auto *Condition = BranchTerm.getCondition();
    assert(Condition);
    assert(Condition->getType()->isIntegerTy(1));
    assert(!isa<ConstantInt>(Condition));

    Pass.emitTerminatorInstrumentation(BranchTerm, *Condition);
  }

  void visitSwitchInst(SwitchInst &SwitchTerm) {
    LLVM_DEBUG(dbgs() << "    Block has a switch terminator.\n");

    auto *Condition = SwitchTerm.getCondition();
    assert(Condition);
    Pass.emitTerminatorInstrumentation(SwitchTerm, *Condition);
  }

  void visitIndirectBrInst(IndirectBrInst &IndirectBrTerm) {
    LLVM_DEBUG(dbgs() << "    Block has an indirectbr terminator.\n");

    auto *Address = IndirectBrTerm.getAddress();
    assert(Address);
    Pass.emitTerminatorInstrumentation(IndirectBrTerm, *Address);
  }
};
} // namespace

char InstCountWrapper::ID = 0;

bool InstCountWrapper::doInitialization(Module &M) {
  auto &C = M.getContext();
  auto VoidTy = Type::getVoidTy(C);

  IdentifierTy = Type::getInt64Ty(C);
  TracedValueTy = Type::getInt64Ty(C);
  CtorDtorTy = FunctionType::get(VoidTy, false);
  CallbackTy = FunctionType::get(VoidTy, {IdentifierTy, TracedValueTy}, false);

  return true;
}

void InstCountWrapper::addDeclarations(Module &M) {
  LLVM_DEBUG(dbgs() << "Emitting declarations.\n");

  auto Ctor = M.getOrInsertFunction(CtorName, CtorDtorTy);
  appendToGlobalCtors(M, cast<Function>(Ctor.getCallee()), 0);

  auto Dtor = M.getOrInsertFunction(DtorName, CtorDtorTy);
  appendToGlobalDtors(M, cast<Function>(Dtor.getCallee()), 0);

  Callback = M.getOrInsertFunction(CallbackName, CallbackTy);
}

void InstCountWrapper::emitTerminatorInstrumentation(
    Instruction &TracedInstruction, Value &TracedValue) const {
  auto TargetInstructionID = IdMap->lookup(&TracedInstruction);
  assert(TargetInstructionID != 0);
  auto TargetInstructionIDValue =
      ConstantInt::get(IdentifierTy, TargetInstructionID);

  IRBuilder<> IRB(&TracedInstruction);
  auto *CastedTracedValue =
      IRB.CreateZExtOrBitCast(&TracedValue, TracedValueTy);
  IRB.CreateCall(Callback, {TargetInstructionIDValue, CastedTracedValue});

  LLVM_DEBUG(dbgs() << "      Emitting call to runtime library.\n");
  ++injectedCalls;
}

void InstCountWrapper::instrumentFunction(Function &F) {
  LLVM_DEBUG(dbgs() << "Instrumenting function: " << F.getName() << '\n');

  for (auto &BB : F) {
    LLVM_DEBUG(dbgs() << "  Block: " << IdMap->lookup(&BB) << '\n');

    auto NumSuccessors = std::distance(succ_begin(&BB), succ_end(&BB));

    if (NumSuccessors <= 1) {
      LLVM_DEBUG(dbgs() << "    Block does not have enough successors.\n");
      continue;
    }

    ConditionVisitor Visitor(*this);
    Visitor.visit(BB.getTerminator());
  }
}

bool InstCountWrapper::runOnModule(Module &M) {
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

static RegisterPass<InstCountWrapper> RegisterInstCountWrapper(
    "inst-count-wrapper",
    "Insert instrumentation for instruction counter pass");

static void registerInstCountWrapperPass(const PassManagerBuilder &,
                                         legacy::PassManagerBase &PM) {
  PM.add(new InstCountWrapper());
}

static RegisterStandardPasses
    RegisterInstCountWrapperOptimizerLast(PassManagerBuilder::EP_OptimizerLast,
                                          registerInstCountWrapperPass);

static RegisterStandardPasses RegisterInstCountWrapperEnabledOnOptLevel0(
    PassManagerBuilder::EP_EnabledOnOptLevel0, registerInstCountWrapperPass);
