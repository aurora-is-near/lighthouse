use crate::{
    test_utils::TestRandom, Address, BeaconState, ChainSpec, Epoch, EthSpec, Hash256,
    PublicKeyBytes,
};
use serde_derive::{Deserialize, Serialize};
use ssz_derive::{Decode, Encode};
use test_random_derive::TestRandom;
use tree_hash_derive::TreeHash;

/// Information about a `BeaconChain` validator.
///
/// Spec v0.12.1
#[cfg_attr(feature = "arbitrary-fuzz", derive(arbitrary::Arbitrary))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode, TestRandom, TreeHash)]
pub struct Validator {
    pub pubkey: PublicKeyBytes,
    pub withdrawal_credentials: Hash256,
    #[serde(with = "eth2_serde_utils::quoted_u64")]
    pub effective_balance: u64,
    pub slashed: bool,
    pub activation_eligibility_epoch: Epoch,
    pub activation_epoch: Epoch,
    pub exit_epoch: Epoch,
    pub withdrawable_epoch: Epoch,
}

impl Validator {
    /// Returns `true` if the validator is considered active at some epoch.
    pub fn is_active_at(&self, epoch: Epoch) -> bool {
        self.activation_epoch <= epoch && epoch < self.exit_epoch
    }

    /// Returns `true` if the validator is slashable at some epoch.
    pub fn is_slashable_at(&self, epoch: Epoch) -> bool {
        !self.slashed && self.activation_epoch <= epoch && epoch < self.withdrawable_epoch
    }

    /// Returns `true` if the validator is considered exited at some epoch.
    pub fn is_exited_at(&self, epoch: Epoch) -> bool {
        self.exit_epoch <= epoch
    }

    /// Returns `true` if the validator is able to withdraw at some epoch.
    pub fn is_withdrawable_at(&self, epoch: Epoch) -> bool {
        epoch >= self.withdrawable_epoch
    }

    /// Returns `true` if the validator is eligible to join the activation queue.
    ///
    /// Spec v0.12.1
    pub fn is_eligible_for_activation_queue(&self, spec: &ChainSpec) -> bool {
        self.activation_eligibility_epoch == spec.far_future_epoch
            && self.effective_balance == spec.max_effective_balance
    }

    /// Returns `true` if the validator is eligible to be activated.
    ///
    /// Spec v0.12.1
    pub fn is_eligible_for_activation<E: EthSpec>(
        &self,
        state: &BeaconState<E>,
        spec: &ChainSpec,
    ) -> bool {
        // Placement in queue is finalized
        self.activation_eligibility_epoch <= state.finalized_checkpoint().epoch
        // Has not yet been activated
        && self.activation_epoch == spec.far_future_epoch
    }

    /// Returns `true` if the validator has eth1 withdrawal credential
    pub fn has_eth1_withdrawal_credential(&self, spec: &ChainSpec) -> bool {
        self.withdrawal_credentials
            .as_bytes()
            .first()
            .map(|byte| *byte == spec.eth1_address_withdrawal_prefix_byte)
            .unwrap_or(false)
    }

    /// Get the eth1 withdrawal address if this validator has one initialized.
    pub fn get_eth1_withdrawal_address(&self, spec: &ChainSpec) -> Option<Address> {
        self.has_eth1_withdrawal_credential(spec)
            .then(|| {
                self.withdrawal_credentials
                    .as_bytes()
                    .get(12..)
                    .map(Address::from_slice)
            })
            .flatten()
    }

    /// Changes withdrawal credentials to  the provided eth1 execution address
    ///
    /// WARNING: this function does NO VALIDATION - it just does it!
    pub fn change_withdrawal_credentials(&mut self, execution_address: &Address, spec: &ChainSpec) {
        let mut bytes = [0u8; 32];
        bytes[0] = spec.eth1_address_withdrawal_prefix_byte;
        bytes[12..].copy_from_slice(execution_address.as_bytes());
        self.withdrawal_credentials = Hash256::from(bytes);
    }

    /// Returns `true` if the validator is fully withdrawable at some epoch
    pub fn is_fully_withdrawable_at(&self, balance: u64, epoch: Epoch, spec: &ChainSpec) -> bool {
        self.has_eth1_withdrawal_credential(spec) && self.withdrawable_epoch <= epoch && balance > 0
    }

    /// Returns `true` if the validator is partially withdrawable
    pub fn is_partially_withdrawable_validator(&self, balance: u64, spec: &ChainSpec) -> bool {
        self.has_eth1_withdrawal_credential(spec)
            && self.effective_balance == spec.max_effective_balance
            && balance > spec.max_effective_balance
    }
}

impl Default for Validator {
    /// Yields a "default" `Validator`. Primarily used for testing.
    fn default() -> Self {
        Self {
            pubkey: PublicKeyBytes::empty(),
            withdrawal_credentials: Hash256::default(),
            activation_eligibility_epoch: Epoch::from(std::u64::MAX),
            activation_epoch: Epoch::from(std::u64::MAX),
            exit_epoch: Epoch::from(std::u64::MAX),
            withdrawable_epoch: Epoch::from(std::u64::MAX),
            slashed: false,
            effective_balance: std::u64::MAX,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default() {
        let v = Validator::default();

        let epoch = Epoch::new(0);

        assert!(!v.is_active_at(epoch));
        assert!(!v.is_exited_at(epoch));
        assert!(!v.is_withdrawable_at(epoch));
        assert!(!v.slashed);
    }

    #[test]
    fn is_active_at() {
        let epoch = Epoch::new(10);

        let v = Validator {
            activation_epoch: epoch,
            ..Validator::default()
        };

        assert!(!v.is_active_at(epoch - 1));
        assert!(v.is_active_at(epoch));
        assert!(v.is_active_at(epoch + 1));
    }

    #[test]
    fn is_exited_at() {
        let epoch = Epoch::new(10);

        let v = Validator {
            exit_epoch: epoch,
            ..Validator::default()
        };

        assert!(!v.is_exited_at(epoch - 1));
        assert!(v.is_exited_at(epoch));
        assert!(v.is_exited_at(epoch + 1));
    }

    #[test]
    fn is_withdrawable_at() {
        let epoch = Epoch::new(10);

        let v = Validator {
            withdrawable_epoch: epoch,
            ..Validator::default()
        };

        assert!(!v.is_withdrawable_at(epoch - 1));
        assert!(v.is_withdrawable_at(epoch));
        assert!(v.is_withdrawable_at(epoch + 1));
    }

    ssz_and_tree_hash_tests!(Validator);
}
