imports!();

use crate::fund_module::*;
use crate::types::fund_type::*;


/// Deals with storage data about delegators.
#[elrond_wasm_derive::module(FundTransformationsModuleImpl)]
pub trait FundTransformationsModule {

    #[module(FundModuleImpl)]
    fn fund_module(&self) -> FundModuleImpl<T, BigInt, BigUint>;

    #[module(FundTransformationsModuleImpl)]
    fn fund_transf_module(&self) -> FundTransformationsModuleImpl<T, BigInt, BigUint>;

    fn create_free_stake(&self, user_id: usize, balance: BigUint) {
        self.fund_module().create_fund(user_id, FundDescription::Waiting, balance);
    }

    fn liquidate_free_stake(&self, user_id: usize, amount: &mut BigUint) {
        // first withdraw from withdraw-only inactive stake
        self.fund_module().destroy_max_for_user(
            amount,
            user_id,
            FundType::WithdrawOnly);

        // if that is not enough, retrieve proper inactive stake
        if *amount > 0 {
            self.fund_module().destroy_max_for_user(
                amount,
                user_id,
                FundType::Waiting);
        }
    }

    fn activate_start_transf(&self, amount: &mut BigUint) -> SCResult<()> {
        let _ = sc_try!(self.fund_module().split_convert_max_by_type(
            Some(amount),
            FundType::Waiting,
            |_, _| Some(FundDescription::PendingActivation)
        ));
        if *amount > 0 {
            return sc_error!("not enough inactive stake");
        }

        Ok(())
    }

    fn activate_finish_ok_transf(&self, amount: &mut BigUint) -> SCResult<()> {
        let _ = sc_try!(self.fund_module().split_convert_max_by_type(
            Some(amount),
            FundType::PendingActivation,
            |_, _| Some(FundDescription::Active)
        ));
        if *amount > 0 {
            return sc_error!("not enough stake pending activation");
        }

        Ok(())
    }

    fn activate_finish_fail_transf(&self, amount: &mut BigUint) -> SCResult<()> {
        let _ = sc_try!(self.fund_module().split_convert_max_by_type(
            Some(amount),
            FundType::PendingActivation,
            |_, _| Some(FundDescription::ActivationFailed)
        ));
        if *amount > 0 {
            return sc_error!("not enough stake pending activation");
        }

        Ok(())
    }

    fn unstake_transf(&self, unstake_user_id: usize, amount: &BigUint) -> SCResult<()> {
        let mut amount_to_unstake = amount.clone();
        let current_bl_nonce = self.get_block_nonce();
        let _ = sc_try!(self.fund_module().split_convert_max_by_user(
            Some(&mut amount_to_unstake),
            unstake_user_id,
            FundType::Active,
            |_| Some(FundDescription::UnStaked{ created: current_bl_nonce })
        ));
        if amount_to_unstake > 0 {
            return sc_error!("cannot offer more than the user active stake");
        }

        Ok(())
    }

    fn swap_active_with_waiting_transf(&self, unstake_user_id: usize, amount: &BigUint) -> SCResult<()> {
        // convert active stake -> deferred payment (seller)
        let mut unstaked_to_convert = amount.clone();
        let current_bl_nonce = self.get_block_nonce();
        let _ = sc_try!(self.fund_module().split_convert_max_by_user(
            Some(&mut unstaked_to_convert),
            unstake_user_id,
            FundType::Active,
            |_| Some(FundDescription::DeferredPayment{ created: current_bl_nonce })
        ));

        // convert inactive -> active (buyer)
        let mut stake_to_activate = amount.clone();
        let _ = sc_try!(self.fund_module().split_convert_max_by_type(
            Some(&mut stake_to_activate),
            FundType::Waiting,
            |_, _| Some(FundDescription::Active)
        ));

        Ok(())
    }

    fn eligible_deferred_payment(&self, 
        user_id: usize, 
        n_blocks_before_claim: u64) -> BigUint {

        let current_bl_nonce = self.get_block_nonce();
        self.fund_module().query_sum_funds_by_user_type(
            user_id,
            FundType::DeferredPayment,
            |fund_desc| {
                if let FundDescription::DeferredPayment{ created } = fund_desc {
                    current_bl_nonce > created + n_blocks_before_claim 
                } else {
                    false
                }
            }
        )
    }

    fn claim_all_eligible_deferred_payments(&self,
        user_id: usize,
        n_blocks_before_claim: u64) -> SCResult<BigUint> {
        
        let current_bl_nonce = self.get_block_nonce();
        self.fund_module().split_convert_max_by_user(
            None,
            user_id,
            FundType::DeferredPayment,
            |fund_desc| {
                if let FundDescription::DeferredPayment{ created } = fund_desc {
                    if current_bl_nonce > created + n_blocks_before_claim {
                        return Some(FundDescription::WithdrawOnly)
                    }
                }
                None
            }
        )
    }

    fn node_unbond_transf(&self, amount: &mut BigUint, block_nonce: u64) -> SCResult<()> {
        let _ = sc_try!(self.fund_module().split_convert_max_by_type(
            Some(amount),
            FundType::UnStaked,
            |_, _| Some(FundDescription::DeferredPayment{ created: block_nonce })
        ));

        if *amount > 0 {
            let _ = sc_try!(self.fund_module().split_convert_max_by_type(
                Some(amount),
                FundType::Active,
                |_, _| Some(FundDescription::DeferredPayment{ created: block_nonce })
            ));
        }

        Ok(())
    }

    fn claim_activation_failed_transf(&self, amount: &mut BigUint) -> SCResult<()> {
        let _ = sc_try!(self.fund_module().split_convert_max_by_type(
            Some(amount),
            FundType::ActivationFailed,
            |_, _| Some(FundDescription::Waiting)
        ));
        if *amount > 0 {
            return sc_error!("not enough stake activation failed");
        }

        Ok(())
    }
}