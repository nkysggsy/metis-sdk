use reth::api::NodeTypes;
use reth::builder::BuilderContext;
use reth::builder::components::ExecutorBuilder;
use reth_evm::block::{BlockExecutor, BlockExecutorFactory};
use reth_evm::eth::EthBlockExecutor;
use reth_evm::{ConfigureEvm, execute::BlockExecutor as RethBlockExecutor, InspectorFor};
use reth_evm::{Evm, EvmEnv};
use reth_evm_ethereum::RethReceiptBuilder;
use reth_node_api::FullNodeTypes;
use reth_optimism_chainspec::OpChainSpec;
use reth_optimism_evm::{OpEvmConfig, OpRethReceiptBuilder};
use reth_optimism_primitives::{OpPrimitives, OpReceipt, OpTransactionSigned};
use std::fmt::Debug;
use std::sync::Arc;
use alloy_evm::op_revm::OpSpecId;
use alloy_consensus::{Block, Header};
use alloy_evm::block::BlockExecutorFor;
use alloy_op_evm::{OpBlockExecutionCtx, OpBlockExecutor, OpEvm, OpEvmFactory};
use reth_primitives::SealedBlock;
use reth_primitives_traits::SealedHeader;
use revm::Database;
use revm::database::State;

/// Ethereum-related EVM configuration with the parallel executor.
#[derive(Debug, Clone)]
pub struct OpParallelEvmConfig {
    pub inner: OpEvmConfig,
}

impl OpParallelEvmConfig {
    pub fn new(chain_spec: Arc<OpChainSpec>) -> Self {
        Self {
            inner: OpEvmConfig::optimism(chain_spec),
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
        self.inner.block_assembler()
    }

    fn evm_env(&self, header: &Header) -> EvmEnv<OpSpecId> {
        self.inner.evm_env(header)
    }

    fn next_evm_env(
        &self,
        parent: &Header,
        attributes: &Self::NextBlockEnvCtx,
    ) -> Result<EvmEnv<OpSpecId>, Self::Error> {
        self.inner.next_evm_env(parent, attributes)
    }

    fn context_for_block(
        &self,
        block: &SealedBlock<Block<OpTransactionSigned>>,
    ) -> OpBlockExecutionCtx {
        self.inner.context_for_block(block)
    }

    fn context_for_next_block(
        &self,
        parent: &SealedHeader,
        attributes: Self::NextBlockEnvCtx,
    ) -> OpBlockExecutionCtx {
        self.inner.context_for_next_block(parent, attributes)
    }
}

impl BlockExecutorFactory for OpParallelEvmConfig {
    type EvmFactory = OpEvmFactory;
    type ExecutionCtx<'a> = OpBlockExecutionCtx;
    type Transaction = OpTransactionSigned;
    type Receipt = OpReceipt;

    fn evm_factory(&self) -> &Self::EvmFactory {
        self.inner.evm_factory()
    }

    fn create_executor<'a, DB, I>(
        &'a self,
        evm: OpEvm<&'a mut State<DB>, I>,
        ctx: OpBlockExecutionCtx,
    ) -> impl BlockExecutorFor<'a, Self, DB, I>
    where
        DB: Database + 'a,
        I: InspectorFor<Self, &'a mut State<DB>> + 'a,
    {
        OPParallelBlockExecutor {
            executor: OpBlockExecutor::new(
                evm,
                ctx,
                self.inner.chain_spec().clone(),
                *self.inner.executor_factory.receipt_builder(),
            ),
        }
    }
}



impl<Node> ExecutorBuilder<Node> for OpParallelExecutorBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = OpChainSpec, Primitives = OpPrimitives>>,
{
    type EVM = Arc<OpEvmConfig>;

    async fn build_evm(self, ctx: &BuilderContext<Node>) -> eyre::Result<Self::EVM> {
        Ok(Arc::new(OpEvmConfig::optimism(ctx.chain_spec())))
    }
}

/// Parallel block executor for Optimism.
pub struct OPParallelBlockExecutor<Evm> {
    /// Eth original executor.
    executor: OpBlockExecutor<Evm, OpRethReceiptBuilder, Arc<OpChainSpec>>,
}



//  /Users/ysg/Github/reth/examples/custom-node/src/evm.rs
