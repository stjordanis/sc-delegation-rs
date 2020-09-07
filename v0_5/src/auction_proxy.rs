
imports!();

use node_storage::types::*;

#[elrond_wasm_derive::callable(AuctionProxy)]
pub trait Auction {
    #[payable]
    #[callback(auction_stake_callback)]
    fn stake(&self,
        #[callback_arg] node_ids: Vec<usize>,
        num_nodes: usize,
        #[multi(2*num_nodes)] bls_keys_signatures: VarArgs<Vec<u8>>,
        #[payment] payment: &BigUint);

    #[callback(auction_unstake_callback)]
    fn unStake(&self,
        #[callback_arg] node_ids: Vec<usize>,
        #[var_args] bls_keys: VarArgs<BLSKey>);

    #[callback(auction_unbond_callback)]
    fn unBond(&self,
        #[callback_arg] node_ids: Vec<usize>,
        #[var_args] bls_keys_signatures: VarArgs<BLSKey>);

    #[callback(auction_claim_callback)]
    fn claim(&self,
        #[callback_arg] node_ids: Vec<usize>);

    #[payable]
    fn unJail(&self,
        #[var_args] bls_keys: VarArgs<BLSKey>,
        #[payment] fine_payment: &BigUint);
}
