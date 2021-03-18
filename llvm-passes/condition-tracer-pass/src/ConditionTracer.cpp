#include "collabfuzz/IDAssigner.h"

#include "llvm/ADT/Statistic.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/InstIterator.h"
#include "llvm/IR/InstVisitor.h"
#include "llvm/IR/InstrTypes.h"
#include "llvm/IR/LegacyPassManager.h"
#include "llvm/IR/Module.h"
#include "llvm/Pass.h"
#include "llvm/Support/Debug.h"
#include "llvm/Support/FormatVariadic.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Transforms/IPO/PassManagerBuilder.h"
#include "llvm/Transforms/Utils/BasicBlockUtils.h"
#include "llvm/Transforms/Utils/ModuleUtils.h"

#define DEBUG_TYPE "cond-tracer"

using namespace llvm;
using collabfuzz::IDAssigner;

STATISTIC(injectedCalls, "Number of injected calls");

namespace {
class ConditionTracer : public ModulePass {
  friend class ConditionVisitor;

  const char *const CtorName = "__cond_tracer_create";
  const char *const DtorName = "__cond_tracer_destroy";
  const char *const CallbackName = "__cond_tracer_trace";

  IntegerType *InstIdTy;
  IntegerType *CaseIdTy;
  FunctionType *CtorDtorTy;
  FunctionType *CallbackTy;

  FunctionCallee Callback;
  const IDAssigner::IdentifiersMap *IdMap;

  void addDeclarations(Module &M);
  void instrumentFunction(Function &F);

public:
  static char ID;
  ConditionTracer() : ModulePass(ID) {}

  void getAnalysisUsage(AnalysisUsage &AU) const override {
    AU.addRequired<IDAssigner>();
  }

  bool doInitialization(Module &M) override;
  bool runOnModule(Module &M) override;
};

class ConditionVisitor : public InstVisitor<ConditionVisitor> {
  ConditionTracer &Pass;

  void handleSwitchCase(BasicBlock *Successor, ConstantInt *CaseIndexValue,
                        ConstantInt *TotalCasesValue, ConstantInt *InstIdValue,
                        SwitchInst &Switch);

public:
  ConditionVisitor(ConditionTracer &Pass) : Pass(Pass){};

  void visitSwitchInst(SwitchInst &I);
  void visitBranchInst(BranchInst &I);
};
} // namespace

char ConditionTracer::ID = 0;

bool ConditionTracer::doInitialization(Module &M) {
  auto &C = M.getContext();
  auto VoidTy = Type::getVoidTy(C);

  InstIdTy = Type::getInt64Ty(C);
  CaseIdTy = Type::getInt64Ty(C);
  CtorDtorTy = FunctionType::get(VoidTy, false);
  CallbackTy = FunctionType::get(VoidTy, {InstIdTy, CaseIdTy, CaseIdTy}, false);

  return true;
}

void ConditionTracer::addDeclarations(Module &M) {
  LLVM_DEBUG(dbgs() << "Emitting declarations.\n");

  auto Ctor = M.getOrInsertFunction(CtorName, CtorDtorTy);
  appendToGlobalCtors(M, cast<Function>(Ctor.getCallee()), 0);

  auto Dtor = M.getOrInsertFunction(DtorName, CtorDtorTy);
  appendToGlobalDtors(M, cast<Function>(Dtor.getCallee()), 0);

  Callback = M.getOrInsertFunction(CallbackName, CallbackTy);
}

void ConditionTracer::instrumentFunction(Function &F) {
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

bool ConditionTracer::runOnModule(Module &M) {
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

void ConditionVisitor::handleSwitchCase(BasicBlock *Successor,
                                        ConstantInt *CaseIndexValue,
                                        ConstantInt *TotalCasesValue,
                                        ConstantInt *InstIdValue,
                                        SwitchInst &Switch) {
  // In case the edge going out of the switch is a critical one, we need to
  // split it and insert the callback in the newly created block.
  BasicBlock *TargetBlock = nullptr;
  if (BasicBlock *CritBlock =
          SplitCriticalEdge(Switch.getParent(), Successor)) {
    TargetBlock = CritBlock;
  } else {
    TargetBlock = Successor;
  }

  IRBuilder<> IRB(TargetBlock, TargetBlock->getFirstInsertionPt());
  IRB.CreateCall(Pass.Callback, {InstIdValue, TotalCasesValue, CaseIndexValue});
  ++injectedCalls;
}

void ConditionVisitor::visitSwitchInst(SwitchInst &SwitchTerm) {
  auto InstId = Pass.IdMap->lookup(&SwitchTerm);
  assert(InstId != 0);
  auto InstIdValue = ConstantInt::get(Pass.InstIdTy, InstId);

  auto TotalCases = SwitchTerm.getNumCases() + 1; // + 1 for the default case
  auto TotalCasesValue = ConstantInt::get(Pass.CaseIdTy, TotalCases);

  // In order to avoid having to keep track of the values of the various cases
  // on the runtime side, the callback is pushed to the successor block for each
  // case. In this way, the program does the case matching for us.
  for (auto &SwitchCase : SwitchTerm.cases()) {
    auto Successor = SwitchCase.getCaseSuccessor();
    auto CaseIndex = SwitchCase.getCaseIndex() + 1; // 0 is the default case
    auto CaseIndexValue = ConstantInt::get(Pass.CaseIdTy, CaseIndex);
    handleSwitchCase(Successor, CaseIndexValue, TotalCasesValue, InstIdValue,
                     SwitchTerm);
  }

  // The default case is not present in `cases`, so it needs to be handled
  // separately.
  auto DefaultCase = *SwitchTerm.case_default();
  auto DefaultSuccessor = DefaultCase.getCaseSuccessor();
  auto DefaultIndexValue = ConstantInt::get(Pass.CaseIdTy, 0);
  handleSwitchCase(DefaultSuccessor, DefaultIndexValue, TotalCasesValue,
                   InstIdValue, SwitchTerm);
}

void ConditionVisitor::visitBranchInst(BranchInst &BranchTerm) {
  assert(BranchTerm.isConditional());

  auto *Condition = BranchTerm.getCondition();
  assert(Condition);
  assert(Condition->getType()->isIntegerTy(1));
  assert(!isa<ConstantInt>(Condition));

  auto InstId = Pass.IdMap->lookup(&BranchTerm);
  assert(InstId != 0);
  LLVM_DEBUG(dbgs() << "br cond: "; dbgs().write_hex(InstId); dbgs() << '\n');

  auto InstIdValue = ConstantInt::get(Pass.InstIdTy, InstId);
  auto TotalCasesValue = ConstantInt::get(Pass.CaseIdTy, 2);

  IRBuilder<> IRB(&BranchTerm);
  auto *ExtCondition = IRB.CreateZExtOrBitCast(Condition, Pass.CaseIdTy);
  IRB.CreateCall(Pass.Callback, {InstIdValue, TotalCasesValue, ExtCondition});

  ++injectedCalls;
}

static RegisterPass<ConditionTracer> X1{
    "cond-tracer", "Insert instrumentation for condition tracing", false,
    false};

static void registerConditionTracerPass(const PassManagerBuilder &,
                                        legacy::PassManagerBase &PM) {
  PM.add(new ConditionTracer());
}

static RegisterStandardPasses X2{PassManagerBuilder::EP_OptimizerLast,
                                 registerConditionTracerPass};

static RegisterStandardPasses X3{PassManagerBuilder::EP_EnabledOnOptLevel0,
                                 registerConditionTracerPass};
