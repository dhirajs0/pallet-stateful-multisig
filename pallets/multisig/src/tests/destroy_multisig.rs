use crate as pallet_multisig;
use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};
use sp_arithmetic::traits::Saturating;

/// Tests for the destroy_multisig extrinsic
/// This module tests the destruction of multisig accounts, including proper storage cleanup,
/// deposit release, and various failure cases.

#[test]
fn destroy_multisig_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let owners = vec![1, 2, 3];
        let threshold = 2;
        let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

        // Create multisig
        assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
            RuntimeOrigin::signed(1),
            owners.clone(),
            threshold
        ));

        // Get initial balance of creator
        let initial_balance = Balances::free_balance(&1);

        // Destroy multisig
        assert_ok!(pallet_multisig::Pallet::<Test>::destroy_multisig(
            RuntimeOrigin::signed(1),
            multisig
        ));

        // Verify multisig is removed from storage
        assert!(Multisigs::<Test>::get(&multisig).is_none());

        // Verify owner mappings are removed
        for owner in owners.iter() {
            assert!(!MultisigOwners::<Test>::contains_key(&multisig, owner));
        }

        // Verify deposit is released back to creator
        let final_balance = Balances::free_balance(&1);
        let multisig_data = Multisigs::<Test>::get(&multisig).unwrap();
        assert_eq!(final_balance, initial_balance.saturating_add(multisig_data.deposit));

        // Verify event
        System::assert_last_event(Event::MultisigDestroyed { multisig, approving_account: 1 }.into());
    });
}

#[test]
fn destroy_multisig_fails_for_non_owner() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let owners = vec![1, 2, 3];
        let threshold = 2;
        let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

        // Create multisig
        assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
            RuntimeOrigin::signed(1),
            owners.clone(),
            threshold
        ));

        // Try to destroy from non-owner account
        assert_noop!(
            pallet_multisig::Pallet::<Test>::destroy_multisig(
                RuntimeOrigin::signed(4),
                multisig
            ),
            Error::<Test>::NotAllowedToDissolve
        );
    });
}

#[test]
fn destroy_multisig_fails_for_nonexistent_multisig() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

        // Try to destroy non-existent multisig
        assert_noop!(
            pallet_multisig::Pallet::<Test>::destroy_multisig(
                RuntimeOrigin::signed(1),
                multisig
            ),
            Error::<Test>::MultisigNotFound
        );
    });
}

#[test]
fn destroy_multisig_cleans_up_transactions() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let owners = vec![1, 2, 3];
        let threshold = 2;
        let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

        // Create multisig
        assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
            RuntimeOrigin::signed(1),
            owners.clone(),
            threshold
        ));

        // Submit a transaction
        let call = Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));
        assert_ok!(pallet_multisig::Pallet::<Test>::submit_transaction(
            RuntimeOrigin::signed(1),
            multisig,
            call
        ));

        // Confirm the transaction
        assert_ok!(pallet_multisig::Pallet::<Test>::confirm_transaction(
            RuntimeOrigin::signed(2),
            multisig,
            0
        ));

        // Destroy multisig
        assert_ok!(pallet_multisig::Pallet::<Test>::destroy_multisig(
            RuntimeOrigin::signed(1),
            multisig
        ));

        // Verify transactions are removed
        assert!(Transactions::<Test>::get(&multisig, 0).is_none());

        // Verify confirmations are removed
        for owner in owners.iter() {
            assert!(!Confirmations::<Test>::contains_key(&multisig, (0, owner)));
        }
    });
} 