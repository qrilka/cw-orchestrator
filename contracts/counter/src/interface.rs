use cw_orch::daemon::queriers::Node;
// ANCHOR: custom_interface
use cw_orch::{interface, prelude::*};

use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

pub const CONTRACT_ID: &str = "counter_contract";

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, id = CONTRACT_ID)]
pub struct CounterContract;

impl<Chain: CwEnv> Uploadable for CounterContract<Chain> {
    /// Return the path to the wasm file corresponding to the contract
    fn wasm(&self) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("counter_contract")
            .unwrap()
    }
    /// Returns a CosmWasm contract wrapper
    fn wrapper(&self) -> Box<dyn MockContract<Empty>> {
        Box::new(
            ContractWrapper::new_with_empty(
                crate::contract::execute,
                crate::contract::instantiate,
                crate::contract::query,
            )
            .with_migrate(crate::contract::migrate),
        )
    }
}
// ANCHOR_END: custom_interface

use cw_orch::anyhow::Result;

// ANCHOR: daemon
impl CounterContract<Daemon> {
    /// Deploys the counter contract at a specific block height
    pub fn await_launch(&self) -> Result<()> {
        let daemon = self.get_chain();

        // Get the node query client, there are a lot of other clients available.
        let node: Node = daemon.querier();
        let mut latest_block = node.latest_block().unwrap();

        while latest_block.height < 100 {
            // wait for the next block
            daemon.next_block().unwrap();
            latest_block = node.latest_block().unwrap();
        }

        let contract = CounterContract::new(daemon.clone());

        // Upload the contract
        contract.upload().unwrap();

        // Instantiate the contract
        let msg = InstantiateMsg { count: 1i32 };
        contract.instantiate(&msg, None, None).unwrap();

        Ok(())
    }
}
// ANCHOR_END: daemon

// ANCHOR: cli
use cw_orch_contract_cli::OrchCliResult;

impl cw_orch_contract_cli::CwCliAddons<Empty> for CounterContract<cw_orch::prelude::Daemon> {
    fn addons(&mut self, _context: Empty) -> OrchCliResult<()>
    where
        Self: cw_orch::prelude::ContractInstance<cw_orch::prelude::Daemon>,
    {
        Ok(())
    }
}
// ANCHOR_END
