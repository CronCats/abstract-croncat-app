use abstract_core::objects::module::ModuleId;
use abstract_sdk::{
    features::{AccountIdentification, Dependencies},
    AbstractSdkResult,
};
use abstract_sdk::{AppInterface, ModuleInterface};
use cosmwasm_std::{Addr, CosmosMsg, Deps};
use croncat_integration_utils::CronCatTaskRequest;
use croncat_sdk_manager::types::TaskBalanceResponse;
use croncat_sdk_tasks::types::TaskResponse;
use cw_asset::AssetListUnchecked;

use crate::contract::CRONCAT_ID;
use crate::msg::{AppExecuteMsg, AppQueryMsg};

// Entry for the cron_cat factory address, stored in the ANS
pub const CRON_CAT_FACTORY: &str = "croncat:factory";

// API for Abstract SDK users
/// Interact with the cron_cat adapter in your module.
pub trait CronCatInterface: AccountIdentification + Dependencies {
    /// Construct a new cron_cat interface
    fn cron_cat<'a>(&'a self, deps: Deps<'a>) -> CronCat<Self> {
        CronCat {
            base: self,
            deps,
            module_id: CRONCAT_ID,
        }
    }
}

impl<T: AccountIdentification + Dependencies> CronCatInterface for T {}

#[derive(Clone)]
pub struct CronCat<'a, T: CronCatInterface> {
    base: &'a T,
    module_id: ModuleId<'a>,
    deps: Deps<'a>,
}

impl<'a, T: CronCatInterface> CronCat<'a, T> {
    /// Get address of this module
    pub fn module_address(&self) -> AbstractSdkResult<Addr> {
        self.base.modules(self.deps).module_address(self.module_id)
    }
    /// Create task
    /// On success it will return [`croncat_integration_utils::CronCatTaskExecutionInfo`] in reply data,
    /// you can save task_hash or any other useful information in dependent module.
    /// This way you can track which tasks were created only by this module
    pub fn create_task(
        &self,
        task: CronCatTaskRequest,
        assets: AssetListUnchecked,
    ) -> AbstractSdkResult<CosmosMsg> {
        self.base.apps(self.deps).request(
            self.module_id,
            AppExecuteMsg::CreateTask {
                task: Box::new(task),
                assets,
            },
        )
    }

    /// Refill a task's balance messages
    pub fn refill_task(
        &self,
        task_hash: impl Into<String>,
        assets: AssetListUnchecked,
    ) -> AbstractSdkResult<CosmosMsg> {
        self.base.apps(self.deps).request(
            self.module_id,
            AppExecuteMsg::RefillTask {
                task_hash: task_hash.into(),
                assets,
            },
        )
    }

    pub fn remove_task(&self, task_hash: impl Into<String>) -> AbstractSdkResult<CosmosMsg> {
        self.base.apps(self.deps).request(
            self.module_id,
            AppExecuteMsg::RemoveTask {
                task_hash: task_hash.into(),
            },
        )
    }
}

impl<'a, T: CronCatInterface> CronCat<'a, T> {
    /// Task information
    pub fn query_task_information(
        &self,
        task_hash: impl Into<String>,
    ) -> AbstractSdkResult<TaskResponse> {
        self.base.apps(self.deps).query(
            self.module_id,
            AppQueryMsg::TaskInfo {
                task_hash: task_hash.into(),
            },
        )
    }

    /// Task balance
    pub fn query_task_balance(
        &self,
        task_hash: impl Into<String>,
    ) -> AbstractSdkResult<TaskBalanceResponse> {
        self.base.apps(self.deps).query(
            self.module_id,
            AppQueryMsg::TaskBalance {
                task_hash: task_hash.into(),
            },
        )
    }

    /// Active tasks
    pub fn query_active_tasks(
        &self,
        start_after: Option<impl Into<String>>,
        limit: Option<u32>,
    ) -> AbstractSdkResult<Vec<String>> {
        self.base.apps(self.deps).query(
            self.module_id,
            AppQueryMsg::ActiveTasks {
                start_after: start_after.map(Into::into),
                limit,
            },
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::msg::ExecuteMsg;
    use abstract_sdk::mock_module::MockModule;
    use abstract_testing::prelude::TEST_MODULE_ID;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{coins, wasm_execute, BankMsg};
    use croncat_integration_utils::*;
    use cw_asset::AssetList;
    use speculoos::prelude::*;

    const TEST_TASK_HASH: &str = "juno:564d9acab76c256659634415d14625812103bc8e87308c5c3c290045e17";
    #[test]
    fn create_task_msg() {
        let mut deps = mock_dependencies();
        deps.querier = abstract_testing::mock_querier();
        let stub = MockModule::new();
        let mut cron_cat = stub.cron_cat(deps.as_ref());
        cron_cat.module_id = TEST_MODULE_ID;

        let task = CronCatTaskRequest {
            interval: CronCatInterval::Once,
            boundary: None,
            stop_on_fail: false,
            actions: vec![CronCatAction {
                msg: BankMsg::Send {
                    to_address: "bob".to_owned(),
                    amount: coins(10, "juno"),
                }
                .into(),
                gas_limit: None,
            }],
            queries: None,
            transforms: None,
            cw20: None,
        };
        let assets: AssetListUnchecked = AssetList::from(coins(10, "juno")).into();
        let expected = ExecuteMsg::from(AppExecuteMsg::CreateTask {
            task: Box::new(task.clone()),
            assets: assets.clone(),
        });

        let actual = cron_cat.create_task(task, assets);

        assert_that!(actual).is_ok();

        let actual = match actual.unwrap() {
            CosmosMsg::Wasm(msg) => msg,
            _ => panic!("expected wasm msg"),
        };
        let expected = wasm_execute(
            abstract_testing::prelude::TEST_MODULE_ADDRESS,
            &expected,
            vec![],
        )
        .unwrap();

        assert_that!(actual).is_equal_to(expected);
    }

    #[test]
    fn refill_task_msg() {
        let mut deps = mock_dependencies();
        deps.querier = abstract_testing::mock_querier();
        let stub = MockModule::new();
        let mut cron_cat = stub.cron_cat(deps.as_ref());
        cron_cat.module_id = TEST_MODULE_ID;

        let task_hash = TEST_TASK_HASH;

        let assets: AssetListUnchecked = AssetList::from(coins(10, "juno")).into();
        let expected = ExecuteMsg::from(AppExecuteMsg::RefillTask {
            task_hash: task_hash.to_owned(),
            assets: assets.clone(),
        });

        let actual = cron_cat.refill_task(task_hash, assets);

        assert_that!(actual).is_ok();

        let actual = match actual.unwrap() {
            CosmosMsg::Wasm(msg) => msg,
            _ => panic!("expected wasm msg"),
        };
        let expected = wasm_execute(
            abstract_testing::prelude::TEST_MODULE_ADDRESS,
            &expected,
            vec![],
        )
        .unwrap();

        assert_that!(actual).is_equal_to(expected);
    }

    #[test]
    fn remove_task_msg() {
        let mut deps = mock_dependencies();
        deps.querier = abstract_testing::mock_querier();
        let stub = MockModule::new();
        let mut cron_cat = stub.cron_cat(deps.as_ref());
        cron_cat.module_id = TEST_MODULE_ID;

        let task_hash = TEST_TASK_HASH;

        let expected = ExecuteMsg::from(AppExecuteMsg::RemoveTask {
            task_hash: task_hash.to_owned(),
        });

        let actual = cron_cat.remove_task(task_hash);

        assert_that!(actual).is_ok();

        let actual = match actual.unwrap() {
            CosmosMsg::Wasm(msg) => msg,
            _ => panic!("expected wasm msg"),
        };
        let expected = wasm_execute(
            abstract_testing::prelude::TEST_MODULE_ADDRESS,
            &expected,
            vec![],
        )
        .unwrap();

        assert_that!(actual).is_equal_to(expected);
    }
}