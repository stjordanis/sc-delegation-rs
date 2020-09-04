
use crate::auction_proxy::Auction;
use node_storage::types::*;

use crate::events::*;
use node_storage::node_config::*;
use crate::rewards::*;
use crate::settings::*;
use user_fund_storage::user_data::*;
use crate::user_stake::*;

imports!();

#[elrond_wasm_derive::module(NodeActivationModuleImpl)]
pub trait ContractStakeModule {

    #[module(UserDataModuleImpl)]
    fn user_data(&self) -> UserDataModuleImpl<T, BigInt, BigUint>;

    #[module(SettingsModuleImpl)]
    fn settings(&self) -> SettingsModuleImpl<T, BigInt, BigUint>;

    #[module(EventsModuleImpl)]
    fn events(&self) -> EventsModuleImpl<T, BigInt, BigUint>;

    #[module(NodeConfigModuleImpl)]
    fn node_config(&self) -> NodeConfigModuleImpl<T, BigInt, BigUint>;

    #[module(RewardsModuleImpl)]
    fn rewards(&self) -> RewardsModuleImpl<T, BigInt, BigUint>;

    #[module(UserStakeModuleImpl)]
    fn user_stake(&self) -> UserStakeModuleImpl<T, BigInt, BigUint>;


    /// Owner activates specific nodes.
    #[endpoint(stakeNodes)]
    fn stake_nodes(&self, amount_to_stake: BigUint,
            #[var_args] bls_keys: VarArgs<BLSKey>) -> SCResult<()> {

        if !self.settings().owner_called() {
            return sc_error!("caller not allowed to stake node");
        }
        if self.rewards().total_unprotected() < amount_to_stake {
            return sc_error!("not enough funds in contract to stake nodes");
        }

        sc_try!(self.user_stake().validate_owner_stake_share());

        let mut node_ids = Vec::<usize>::with_capacity(bls_keys.len());
        let mut bls_keys_signatures = Vec::<Vec<u8>>::with_capacity(2 * bls_keys.len());

        for bls_key in bls_keys.iter() {
            let node_id = self.node_config().get_node_id(&bls_key);
            require!(node_id != 0, "unknown node provided");

            require!(self.node_config().get_node_state(node_id) == NodeState::Inactive,
                "node must be inactive");

            node_ids.push(node_id);
            bls_keys_signatures.push(bls_key.to_vec());
            bls_keys_signatures.push(self.node_config().get_node_signature(node_id).to_vec());

            self.node_config().set_node_state(node_id, NodeState::PendingActivation);
        }

        self.perform_stake_nodes(node_ids, bls_keys_signatures, amount_to_stake)
    }

    fn perform_stake_nodes(&self, node_ids: Vec<usize>, bls_keys_signatures: Vec<Vec<u8>>, amount_to_stake: BigUint) -> SCResult<()> {
        let num_nodes = node_ids.len();
        // send all stake to auction contract
        let auction_contract_addr = self.settings().get_auction_contract_address();
        let auction_contract = contract_proxy!(self, &auction_contract_addr, Auction);
        auction_contract.stake(
            node_ids, // callback arg
            num_nodes,
            bls_keys_signatures.into(),
            &amount_to_stake);

        Ok(())
    }

    /// Only finalize activation if we got confirmation from the auction contract.
    /// #[callback] can only be declared in lib.rs for the moment.
    fn auction_stake_callback(&self, 
            node_ids: Vec<usize>, // #[callback_arg]
            call_result: AsyncCallResult<VarArgs<BLSStatusMultiArg>>) -> SCResult<()> {

        match call_result {
            AsyncCallResult::Ok(node_status_args) => {
                let (node_ids_ok, node_ids_fail) = self.node_config().split_node_ids_by_err(node_ids, node_status_args);
                sc_try!(self.auction_stake_callback_ok(node_ids_ok));
                sc_try!(self.auction_stake_callback_fail(node_ids_fail, &b"staking failed for some nodes"[..]));
                Ok(())
            },
            AsyncCallResult::Err(error) => {
                self.auction_stake_callback_fail(node_ids, error.err_msg.as_slice())
            }
        }
    }

    fn auction_stake_callback_ok(&self, node_ids: Vec<usize>) -> SCResult<()> {
        if node_ids.is_empty() {
            return Ok(());
        }

        // set nodes to Active
        for &node_id in node_ids.iter() {
            self.node_config().set_node_state(node_id, NodeState::Active);
        }
        
        // log event (no data)
        // TODO: log BLS keys of nodes in data
        self.events().stake_node_ok_event(());

        Ok(())
    }

    fn auction_stake_callback_fail(&self, node_ids: Vec<usize>, err_msg: &[u8]) -> SCResult<()> {
        if node_ids.is_empty() {
            return Ok(());
        }

        // set nodes to ActivationFailed
        for &node_id in node_ids.iter() {
            self.node_config().set_node_state(node_id, NodeState::ActivationFailed);
        }

        // log failure event (no data)
        self.events().stake_node_fail_event(err_msg);

        Ok(())
    }

    // UNSTAKE

    /// Unstakes from the auction smart contract.
    /// The nodes will stop receiving rewards, but stake cannot be yet reclaimed.
    /// This operation is performed by the owner.
    #[endpoint(unStakeNodes)]
    fn unstake_nodes(&self,
            #[var_args] bls_keys: VarArgs<BLSKey>) -> SCResult<()> {

        if !self.settings().owner_called() {
            return sc_error!("caller not allowed to unstake node");
        }

        let mut node_ids = Vec::<usize>::with_capacity(bls_keys.len());
        for bls_key in bls_keys.iter() {
            let node_id = self.node_config().get_node_id(&bls_key);
            node_ids.push(node_id);
        }

        self.perform_unstake_nodes(node_ids, bls_keys.into_vec())
    }

    fn perform_unstake_nodes(&self,
            node_ids: Vec<usize>,
            bls_keys: Vec<BLSKey>) -> SCResult<()> {

        // convert node state to PendingDeactivation
        for &node_id in node_ids.iter() {
            if self.node_config().get_node_state(node_id) != NodeState::Active {
                return sc_error!("node not active");
            }
            self.node_config().set_node_state(node_id, NodeState::PendingDeactivation);
        }

        // send unstake command to Auction SC
        let auction_contract_addr = self.settings().get_auction_contract_address();
        let auction_contract = contract_proxy!(self, &auction_contract_addr, Auction);
        auction_contract.unStake(
            node_ids,
            bls_keys.into());

        Ok(())
    }

    /// Only finalize deactivation if we got confirmation from the auction contract.
    /// #[callback] can only be declared in lib.rs for the moment.
    fn auction_unstake_callback(&self,
            node_ids: Vec<usize>, // #[callback_arg]
            call_result: AsyncCallResult<VarArgs<BLSStatusMultiArg>>) -> SCResult<()> {

        match call_result {
            AsyncCallResult::Ok(node_status_args) => {
                let (node_ids_ok, node_ids_fail) = self.node_config().split_node_ids_by_err(node_ids, node_status_args);
                sc_try!(self.auction_unstake_callback_ok(node_ids_ok));
                sc_try!(self.auction_unstake_callback_fail(node_ids_fail, &b"unstaking failed for some nodes"[..]));
                Ok(())
            },
            AsyncCallResult::Err(error) => {
                self.auction_unstake_callback_fail(node_ids, error.err_msg.as_slice())
            }
        }
    }

    fn auction_unstake_callback_ok(&self,
            node_ids: Vec<usize>) -> SCResult<()> {

        if node_ids.is_empty() {
            return Ok(());
        }

        // set nodes to UnBondPeriod + save current block nonce
        let bl_nonce = self.get_block_nonce();
        for &node_id in node_ids.iter() {
            self.node_config().set_node_state(node_id, NodeState::UnBondPeriod{ started: bl_nonce});
        }

        // log event (no data)
        // TODO: log BLS keys of nodes in data
        self.events().unstake_node_ok_event(());

        Ok(())
    }

    fn auction_unstake_callback_fail(&self, node_ids: Vec<usize>, err_msg: &[u8]) -> SCResult<()> {
        if node_ids.is_empty() {
            return Ok(());
        }

        // revert nodes to Active
        for &node_id in node_ids.iter() {
            self.node_config().set_node_state(node_id, NodeState::Active);
        }

        // log failure event (no data)
        self.events().unstake_node_fail_event(err_msg);

        Ok(())
    }

    // UNBOND
    /// Calls unbond for all nodes that are in the unbond period and are due.
    #[endpoint(unBondNodes)]
    fn unbond_all_available(&self) -> SCResult<()> {
        if !self.settings().caller_can_activate() {
            return sc_error!("caller not allowed to unbond nodes");
        }

        let mut node_id = self.node_config().get_num_nodes();
        let mut node_ids = Vec::<usize>::new();
        let mut bls_keys = Vec::<BLSKey>::new();
        let bl_nonce = self.get_block_nonce();
        let n_blocks_before_unbond = self.settings().get_n_blocks_before_unbond();
        while node_id >= 1 {
            if let NodeState::UnBondPeriod{ started } = self.node_config().get_node_state(node_id) {
                if bl_nonce >= started + n_blocks_before_unbond {
                    self.node_config().set_node_state(node_id, NodeState::PendingUnBond{ unbond_started: started });
                    node_ids.push(node_id);
                    bls_keys.push(self.node_config().get_node_id_to_bls(node_id));
                }
            }

            node_id -= 1;
        }

        if node_ids.is_empty() {
            return Ok(())
        }

        self.perform_unbond(node_ids, bls_keys)
    }

    fn perform_unbond(&self,
        node_ids: Vec<usize>,
        bls_keys: Vec<BLSKey>) -> SCResult<()> {
        
        // send unbond command to Auction SC
        let auction_contract_addr = self.settings().get_auction_contract_address();
        let auction_contract = contract_proxy!(self, &auction_contract_addr, Auction);
        auction_contract.unBond(
            node_ids,
            bls_keys.into());

        Ok(())
    }

    /// Only finalize deactivation if we got confirmation from the auction contract.
    /// #[callback] can only be declared in lib.rs for the moment.
    fn auction_unbond_callback(&self, 
        node_ids: Vec<usize>, // #[callback_arg]
        call_result: AsyncCallResult<VarArgs<BLSStatusMultiArg>>) -> SCResult<()> {

        match call_result {
            AsyncCallResult::Ok(node_status_args) => {
                let (node_ids_ok, node_ids_fail) = self.node_config().split_node_ids_by_err(node_ids, node_status_args);
                sc_try!(self.auction_unbond_callback_ok(node_ids_ok));
                sc_try!(self.auction_unbond_callback_fail(node_ids_fail, &b"unbonding failed for some nodes"[..]));
                Ok(())
            },
            AsyncCallResult::Err(error) => {
                self.auction_unbond_callback_fail(node_ids, error.err_msg.as_slice())
            }
        }
    }

    fn auction_unbond_callback_ok(&self, node_ids: Vec<usize>) -> SCResult<()> {
        if node_ids.is_empty() {
            return Ok(());
        }

        // set nodes to Inactive + reset unstake nonce since it is no longer needed
        for &node_id in node_ids.iter() {
            self.node_config().set_node_state(node_id, NodeState::Inactive);
        }

        // log event (no data)
        // TODO: log BLS keys of nodes in data
        self.events().unbond_node_ok_event(());

        Ok(())
    }

    fn auction_unbond_callback_fail(&self, node_ids: Vec<usize>, err_msg: &[u8]) -> SCResult<()> {
        if node_ids.is_empty() {
            return Ok(());
        }

        // revert nodes to UnBondPeriod
        for &node_id in node_ids.iter() {
            if let NodeState::PendingUnBond{ unbond_started } = self.node_config().get_node_state(node_id) {
                self.node_config().set_node_state(node_id, NodeState::UnBondPeriod{ started: unbond_started });
            } else {
                return sc_error!("node not pending unbond");
            }
        }

        // log failure event (no data)
        self.events().unbond_node_fail_event(err_msg);

        Ok(())
    }

    /// Claims unstaked stake from the auction smart contract.
    #[endpoint(claimInactive)]
    fn claim_inactive_stake(&self) -> SCResult<()> {
        if !self.settings().owner_called() {
            return sc_error!("only owner can activate nodes individually"); 
        }

        let mut node_id = self.node_config().get_num_nodes();
        let mut node_ids = Vec::<usize>::new();
        while node_id >= 1 {
            if self.node_config().get_node_state(node_id) == NodeState::ActivationFailed {
                node_ids.push(node_id);
            }
            node_id -= 1;
        }

        if node_ids.is_empty() {
            return Ok(())
        }

        // send claim command to Auction SC
        let auction_contract_addr = self.settings().get_auction_contract_address();
        let auction_contract = contract_proxy!(self, &auction_contract_addr, Auction);
        auction_contract.claim(node_ids);

        Ok(())
    }

    /// Set nodes and stake to inactive, but only after call to auction claim completed.
    /// #[callback] can only be declared in lib.rs for the moment.
    fn auction_claim_callback(&self,
            node_ids: Vec<usize>, // #[callback_arg]
            call_result: AsyncCallResult<()>) -> SCResult<()> {

        match call_result {
            AsyncCallResult::Ok(()) => {
                // set nodes to Inactive
                for &node_id in node_ids.iter() {
                    self.node_config().set_node_state(node_id, NodeState::Inactive);
                }
            },
            AsyncCallResult::Err(_) => {
            }
        }

        Ok(())
    }

}
