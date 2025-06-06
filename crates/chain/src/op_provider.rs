use std::sync::Arc;
use reth::api::NodeTypes;
use reth::builder::BuilderContext;
use reth::builder::components::ExecutorBuilder;
use reth_node_api::FullNodeTypes;
use reth_optimism_chainspec::OpChainSpec;
use reth_optimism_evm::{OpEvmConfig, OpRethReceiptBuilder};
use reth_optimism_primitives::OpPrimitives;
use alloy_op_evm::OpBlockExecutor;
use alloy_evm::Database;
use alloy_evm::block::{BlockExecutionError, BlockExecutionResult, CommitChanges};
use alloy_evm::eth::dao_fork::DAO_HARDFORK_BENEFICIARY;
use alloy_evm::eth::{dao_fork, eip6110};
use alloy_hardforks::EthereumHardfork;
use metis_primitives::{CfgEnv, ExecutionResult, SpecId, TxEnv};
use reth::providers::BlockExecutionResult as RethBlockExecutionResult;
use reth_evm::block::{BlockExecutor, BlockExecutorFactory, BlockExecutorFor, ExecutableTx};
use reth_evm::eth::spec::EthExecutorSpec;
use reth_evm::eth::{EthBlockExecutionCtx, EthBlockExecutor};
use reth_evm::{ConfigureEvm, OnStateHook, execute::BlockExecutor as RethBlockExecutor};
use reth_evm::{Evm, EvmEnv, EvmFactory, EvmFor, InspectorFor};
use reth_evm_ethereum::EthEvmConfig;
use reth_evm_ethereum::{EthBlockAssembler, RethReceiptBuilder};
use reth_primitives_traits::{SealedBlock, SealedHeader};
use revm::DatabaseCommit;
use std::convert::Infallible;
use std::fmt::Debug;
use std::num::NonZeroUsize;
use std::sync::Arc;
use metis_pe::ParallelExecutor;
use crate::state::StateStorageAdapter;

/// Ethereum-related EVM configuration with the parallel executor.
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