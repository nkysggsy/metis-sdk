use alloy_consensus::{Block, Header};
use alloy_evm::block::{
    BlockExecutionError, BlockExecutionResult, BlockExecutorFor, CommitChanges, ExecutableTx,
    OnStateHook,
};
use alloy_evm::op_revm::{OpSpecId, OpTransaction};
use alloy_op_evm::{OpBlockExecutionCtx, OpBlockExecutor, OpEvm, OpEvmFactory};
use metis_primitives::ExecutionResult;
use reth::api::NodeTypes;
use reth::builder::BuilderContext;
use reth::builder::components::ExecutorBuilder;
use reth::revm::context::TxEnv;
use reth_evm::block::BlockExecutorFactory;
use reth_evm::{ConfigureEvm, InspectorFor, execute::BlockExecutor};
use reth_evm::{Evm, EvmEnv};
use reth_node_api::FullNodeTypes;
use reth_optimism_chainspec::OpChainSpec;
use reth_optimism_evm::{OpEvmConfig, OpRethReceiptBuilder};
use reth_optimism_primitives::{OpPrimitives, OpReceipt, OpTransactionSigned};
use reth_primitives::SealedBlock;
use reth_primitives_traits::SealedHeader;
use revm::Database;
use revm::database::State;
use std::fmt::Debug;
use std::sync::Arc;
use alloy_evm::precompiles::PrecompilesMap;

/// Optimism-related EVM configuration with the parallel executor.
#[derive(Debug, Clone)]
pub struct OpParallelEvmConfig {
    pub config: OpEvmConfig,
}

impl OpParallelEvmConfig {
    pub fn new(chain_spec: Arc<OpChainSpec>) -> Self {
        Self {
            config: OpEvmConfig::optimism(chain_spec),
        }
    }
}

impl ConfigureEvm for OpParallelEvmConfig {
    type Primitives = OpPrimitives;
    type Error = <OpEvmConfig as ConfigureEvm>::Error;
    type NextBlockEnvCtx = <OpEvmConfig as ConfigureEvm>::NextBlockEnvCtx;
    type BlockExecutorFactory = Self;
    type BlockAssembler = <OpEvmConfig as ConfigureEvm>::BlockAssembler;

    fn block_executor_factory(&self) -> &Self::BlockExecutorFactory {
        self
    }

    fn block_assembler(&self) -> &Self::BlockAssembler {
        self.config.block_assembler()
    }

    fn evm_env(&self, header: &Header) -> EvmEnv<OpSpecId> {
        self.config.evm_env(header)
    }

    fn next_evm_env(
        &self,
        parent: &Header,
        attributes: &Self::NextBlockEnvCtx,
    ) -> Result<EvmEnv<OpSpecId>, Self::Error> {
        self.config.next_evm_env(parent, attributes)
    }

    fn context_for_block(
        &self,
        block: &SealedBlock<Block<OpTransactionSigned>>,
    ) -> OpBlockExecutionCtx {
        self.config.context_for_block(block)
    }

    fn context_for_next_block(
        &self,
        parent: &SealedHeader,
        attributes: Self::NextBlockEnvCtx,
    ) -> OpBlockExecutionCtx {
        self.config.context_for_next_block(parent, attributes)
    }
}

impl BlockExecutorFactory for OpParallelEvmConfig {
    type EvmFactory = OpEvmFactory;
    type ExecutionCtx<'a> = OpBlockExecutionCtx;
    type Transaction = OpTransactionSigned;
    type Receipt = OpReceipt;

    fn evm_factory(&self) -> &Self::EvmFactory {
        self.config.evm_factory()
    }

    fn create_executor<'a, DB, I>(
        &'a self,
        evm: OpEvm<&'a mut State<DB>, I, PrecompilesMap>,
        ctx: OpBlockExecutionCtx,
    ) -> impl BlockExecutorFor<'a, Self, DB, I>
    where
        DB: Database + 'a,
        I: InspectorFor<Self, &'a mut State<DB>> + 'a,  <DB as Database>::Error: Send
    {
        OpParallelBlockExecutor {
            inner: OpBlockExecutor::new(
                evm,
                ctx,
                self.config.chain_spec().clone(),
                *self.config.executor_factory.receipt_builder(),
            ),
        }
    }
}

/// Parallel block executor for Optimism.
pub struct OpParallelBlockExecutor<Evm> {
    inner: OpBlockExecutor<Evm, OpRethReceiptBuilder, Arc<OpChainSpec>>,
}

impl<'db, DB, E> BlockExecutor for OpParallelBlockExecutor<E>
where
    DB: Database + 'db,
    E: Evm<DB = &'db mut State<DB>, Tx = OpTransaction<TxEnv>>,
{
    type Transaction = OpTransactionSigned;
    type Receipt = OpReceipt;
    type Evm = E;

    fn apply_pre_execution_changes(&mut self) -> Result<(), BlockExecutionError> {
        self.inner.apply_pre_execution_changes()
    }

    fn execute_transaction_with_commit_condition(
        &mut self,
        tx: impl ExecutableTx<Self>,
        f: impl FnOnce(&ExecutionResult<<Self::Evm as Evm>::HaltReason>) -> CommitChanges,
    ) -> Result<Option<u64>, BlockExecutionError> {
        self.inner.execute_transaction_with_result_closure(tx, f)
    }

    fn finish(
        self,
    ) -> Result<(Self::Evm, BlockExecutionResult<Self::Receipt>), BlockExecutionError> {
        self.inner.finish()
    }

    fn set_state_hook(&mut self, hook: Option<Box<dyn OnStateHook>>) {
        self.inner.set_state_hook(hook)
    }

    fn evm_mut(&mut self) -> &mut Self::Evm {
        self.inner.evm_mut()
    }

    fn evm(&self) -> &Self::Evm {
        self.inner.evm()
    }
}

#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct OpParallelExecutorBuilder;
impl<Node> ExecutorBuilder<Node> for OpParallelExecutorBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = OpChainSpec, Primitives = OpPrimitives>>,
{
    type EVM = Arc<OpEvmConfig>;

    async fn build_evm(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::EVM> {
        Ok(Arc::new(OpEvmConfig::optimism(ctx.chain_spec())))
    }
}

//  /Users/ysg/Github/reth/examples/custom-node/src/evm.rs
