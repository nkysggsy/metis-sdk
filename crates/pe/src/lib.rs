use alloy_evm::EvmEnv;
use bitflags::bitflags;
use revm::{Database, DatabaseRef};
use std::hash::Hash;
use std::sync::Arc;

/// This optimization is desired as we constantly index into many
/// vectors of the block-size size. It can yield up to 5% improvement.
macro_rules! index_mutex {
    ($vec:expr, $index:expr) => {
        // SAFETY: A correct scheduler would not leak indexes larger
        // than the block size, which is the size of all vectors we
        // index via this macro. Otherwise, DO NOT USE!
        unsafe { $vec.get_unchecked($index).lock().unwrap() }
    };
}

bitflags! {
    struct FinishExecFlags: u8 {
        // Do we need to validate from this transaction?
        // The first and lazy transactions don't need validation. Note
        // that this is used to tune the min validation index in the
        // scheduler, meaning a [false] here will still be validated if
        // there was a lower transaction that has broken the preprocessed
        // dependency chain and returned [true]
        const NeedValidation = 0;
        // We need to validate from the next transaction if this execution
        // wrote to a new location.
        const WroteNewLocation = 1;
    }
}

pub trait VmTr<'a, DB2: Database> {
    type DB: DatabaseRef;

    type Evm;

    fn new(
        db: &'a Self::DB,
        mv_memory: &'a MvMemory,
        evm_env: &'a EvmEnv,
        txs: &'a [TxEnv],
        #[cfg(feature = "compiler")] worker: Arc<ExtCompileWorker>,
    ) -> Self;

    fn execute(&self, tx_version: &TxVersion) -> Result<VmExecutionResult, VmExecutionError>;

    fn build_mv_memory(block_env: &BlockEnv, txs: &[TxEnv]) -> MvMemory {
        mv_memory::build_mv_memory(block_env, txs)
    }

    fn build_evm(db: DB2, evm_env: EvmEnv) -> Self::Evm;
}

pub mod dropper;
pub use dropper::AsyncDropper;
pub mod executor;
pub mod mv_memory;
pub use mv_memory::MvMemory;
pub mod scheduler;
pub mod types;
pub use executor::{ParallelExecutor, execute_sequential};
pub use scheduler::{DAGProvider, NormalProvider};
pub use types::*;
pub mod db;
pub use db::InMemoryDB;
pub use metis_primitives::{Account, AccountInfo, AccountState, BlockHashes, Bytecodes};
use metis_primitives::{BlockEnv, TxEnv};
#[cfg(feature = "compiler")]
use metis_vm::ExtCompileWorker;

mod vm;

mod op_executor;
pub use op_executor::OpParallelExecutor;
mod op_vm;
pub mod result;

use crate::result::{VmExecutionError, VmExecutionResult};
pub use result::{
    DBError, ExecutionError, ParallelExecutorError, ParallelExecutorResult, TxExecutionResult,
};
