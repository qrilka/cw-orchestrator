use std::marker::PhantomData;
use std::{cell::RefCell, rc::Rc};

use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{instantiate2_address, Api};
use cosmwasm_std::{to_json_binary, ContractInfoResponse, HexBinary};
use cw_orch_core::{
    contract::interface_traits::{ContractInstance, Uploadable},
    environment::{Querier, QuerierGetter, QueryHandler, StateInterface, TxHandler, WasmQuerier},
    CwEnvError,
};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};

use crate::{core::MockApp, MockBase};

pub struct MockWasmQuerier<A: Api, S: StateInterface> {
    app: Rc<RefCell<MockApp<A>>>,
    _state: PhantomData<S>,
}

impl<A: Api, S: StateInterface> MockWasmQuerier<A, S> {
    fn new(mock: &MockBase<A, S>) -> Self {
        Self {
            app: mock.app.clone(),
            _state: PhantomData,
        }
    }
}

impl<A: Api, S: StateInterface> Querier for MockWasmQuerier<A, S> {
    type Error = CwEnvError;
}

impl<A: Api, S: StateInterface> QuerierGetter<MockWasmQuerier<A, S>> for MockBase<A, S> {
    fn querier(&self) -> MockWasmQuerier<A, S> {
        MockWasmQuerier::new(self)
    }
}

fn code_id_hash<A: Api, S: StateInterface>(
    querier: &MockWasmQuerier<A, S>,
    code_id: u64,
) -> Result<HexBinary, CwEnvError> {
    let code_info = querier.app.borrow().wrap().query_wasm_code_info(code_id)?;
    Ok(code_info.checksum)
}

fn contract_info<A: Api, S: StateInterface>(
    querier: &MockWasmQuerier<A, S>,
    address: impl Into<String>,
) -> Result<ContractInfoResponse, CwEnvError> {
    let info = querier
        .app
        .borrow()
        .wrap()
        .query_wasm_contract_info(address)?;
    Ok(info)
}

fn local_hash<Chain: TxHandler + QueryHandler, T: Uploadable + ContractInstance<Chain>>(
    contract: &T,
) -> Result<HexBinary, CwEnvError> {
    // We return the hashed contract-id.
    // This will cause the logic to never re-upload a contract if it has the same contract-id.
    let hash: [u8; 32] = Sha256::digest(contract.id()).into();
    Ok(hash.into())
}

fn raw_query<A: Api, S: StateInterface>(
    querier: &MockWasmQuerier<A, S>,
    address: impl Into<String>,
    query_data: Vec<u8>,
) -> Result<Vec<u8>, CwEnvError> {
    Ok(querier
        .app
        .borrow()
        .wrap()
        .query(&cosmwasm_std::QueryRequest::Wasm(
            cosmwasm_std::WasmQuery::Raw {
                contract_addr: address.into(),
                key: query_data.into(),
            },
        ))?)
}

fn smart_query<A: Api, S: StateInterface, Q, T>(
    querier: &MockWasmQuerier<A, S>,
    address: impl Into<String>,
    query_data: &Q,
) -> Result<T, CwEnvError>
where
    T: DeserializeOwned,
    Q: Serialize,
{
    Ok(querier
        .app
        .borrow()
        .wrap()
        .query(&cosmwasm_std::QueryRequest::Wasm(
            cosmwasm_std::WasmQuery::Smart {
                contract_addr: address.into(),
                msg: to_json_binary(query_data)?,
            },
        ))?)
}

fn code<A: Api, S: StateInterface>(
    querier: &MockWasmQuerier<A, S>,
    code_id: u64,
) -> Result<cosmwasm_std::CodeInfoResponse, CwEnvError> {
    Ok(querier
        .app
        .borrow()
        .wrap()
        .query(&cosmwasm_std::QueryRequest::Wasm(
            cosmwasm_std::WasmQuery::CodeInfo { code_id },
        ))?)
}

impl<A: Api, S: StateInterface> WasmQuerier for MockWasmQuerier<A, S> {
    type Chain = MockBase<A, S>;
    /// Returns the hex-encoded checksum of the code.
    fn code_id_hash(&self, code_id: u64) -> Result<HexBinary, CwEnvError> {
        code_id_hash(self, code_id)
    }

    /// Returns the code_info structure of the provided contract
    fn contract_info(
        &self,
        address: impl Into<String>,
    ) -> Result<ContractInfoResponse, CwEnvError> {
        contract_info(self, address)
    }

    fn local_hash<T: Uploadable + ContractInstance<Self::Chain>>(
        &self,
        contract: &T,
    ) -> Result<HexBinary, CwEnvError> {
        local_hash(contract)
    }

    fn raw_query(
        &self,
        address: impl Into<String>,
        query_data: Vec<u8>,
    ) -> Result<Vec<u8>, CwEnvError> {
        raw_query(self, address, query_data)
    }

    fn smart_query<Q, T>(&self, address: impl Into<String>, query_data: &Q) -> Result<T, CwEnvError>
    where
        T: DeserializeOwned,
        Q: Serialize,
    {
        smart_query(self, address, query_data)
    }

    fn code(&self, code_id: u64) -> Result<cosmwasm_std::CodeInfoResponse, CwEnvError> {
        code(self, code_id)
    }

    fn instantiate2_addr(
        &self,
        code_id: u64,
        creator: impl Into<String>,
        salt: cosmwasm_std::Binary,
    ) -> Result<String, CwEnvError> {
        // little hack to figure out which instantiate2 generator to use.
        // Without this hack the querier methods can't be implemented on a generic "MockApi<A>"
        const MOCK_ADDR: &str = "cosmos1g0pzl69nr8j7wyxxkzurj808svnrrrxtfl8qqm";

        let mock_canonical = MockApi::default().addr_canonicalize(MOCK_ADDR)?;
        let mock_humanized = self.app.borrow().api().addr_humanize(&mock_canonical);

        if mock_humanized.is_ok() && mock_humanized.unwrap() == MOCK_ADDR {
            // if regular mock
            Ok(format!(
                "contract/{}/{}",
                creator.into(),
                HexBinary::from(salt).to_hex()
            ))
        } else {
            // if bech32 mock
            let checksum = self.code_id_hash(code_id)?;
            let canon_creator = self.app.borrow().api().addr_canonicalize(&creator.into())?;
            let canonical_addr = instantiate2_address(checksum.as_slice(), &canon_creator, &salt)?;
            Ok(self
                .app
                .borrow()
                .api()
                .addr_humanize(&canonical_addr)?
                .to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Addr, Binary, Empty, HexBinary, Response, StdError};
    use cw_multi_test::ContractWrapper;
    use cw_orch_core::environment::{DefaultQueriers, TxHandler, WasmQuerier};

    use crate::{Mock, MockBech32};

    #[test]
    fn bech32_instantiate2() -> anyhow::Result<()> {
        let mock = MockBech32::new("mock");

        // For this instantiate 2, we need a registered code id
        mock.upload_custom(
            "test-contract",
            Box::new(ContractWrapper::new_with_empty(
                |_, _, _, _: Empty| Ok::<_, StdError>(Response::new()),
                |_, _, _, _: Empty| Ok::<_, StdError>(Response::new()),
                |_, _, _: Empty| Ok::<_, StdError>(Binary(b"dummy-response".to_vec())),
            )),
        )?;

        mock.wasm_querier()
            .instantiate2_addr(1, mock.sender(), Binary(b"salt-test".to_vec()))?;

        Ok(())
    }

    #[test]
    fn bech32_instantiate2_contract() -> anyhow::Result<()> {
        let mut mock = MockBech32::new("mock");
        mock.set_sender(Addr::unchecked(
            "mock1pgm8hyk0pvphmlvfjc8wsvk4daluz5tgrw6pu5mfpemk74uxnx9qwrtv4f",
        ));

        // For this instantiate 2, we need a registered code id
        mock.upload_custom(
            "test-contract",
            Box::new(ContractWrapper::new_with_empty(
                |_, _, _, _: Empty| Ok::<_, StdError>(Response::new()),
                |_, _, _, _: Empty| Ok::<_, StdError>(Response::new()),
                |_, _, _: Empty| Ok::<_, StdError>(Binary(b"dummy-response".to_vec())),
            )),
        )?;

        mock.wasm_querier()
            .instantiate2_addr(1, mock.sender(), Binary(b"salt-test".to_vec()))?;

        Ok(())
    }

    #[test]
    fn normal_instantiate2() -> anyhow::Result<()> {
        let mock = Mock::new("sender");

        let addr = mock.wasm_querier().instantiate2_addr(
            0,
            mock.sender(),
            Binary(b"salt-test".to_vec()),
        )?;

        assert_eq!(
            addr,
            format!("contract/sender/{}", HexBinary::from(b"salt-test").to_hex())
        );

        Ok(())
    }
}
