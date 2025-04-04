use crate as pallet_multisig;
use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};

/// Tests for the create_multisig extrinsic
/// This module tests the creation of multisig accounts with various parameters
/// and verifies proper storage updates and event emissions.

#[test]
fn create_multisig_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 2, 3];
		let threshold = 2;

		// Get initial balance of creator
		let initial_balance = Balances::free_balance(&1);

		assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
			RuntimeOrigin::signed(1),
			owners.clone(),
			threshold
		));

		// Verify multisig was created
		let multisig =
			Multisigs::<Test>::get(&pallet_multisig::Pallet::<Test>::multi_account_id(0)).unwrap();
		assert_eq!(multisig.owners, owners);
		assert_eq!(multisig.required, threshold);
		assert_eq!(multisig.transaction_count, 0);
		assert_eq!(multisig.creator, 1);

		// Calculate deposit using safe math
		let base_deposit = 100u128;
		let factor_per_owner = 50u128;
		let expected_deposit =
			base_deposit.saturating_add(factor_per_owner.saturating_mul(owners.len() as u128));
		assert_eq!(multisig.deposit, expected_deposit);

		// Verify owner mappings
		for owner in owners.iter() {
			assert_eq!(
				MultisigOwners::<Test>::get(
					&pallet_multisig::Pallet::<Test>::multi_account_id(0),
					owner
				),
				Some(true)
			);
		}

		// Verify deposit was held from creator using safe math
		let final_balance = Balances::free_balance(&1);
		assert_eq!(final_balance, initial_balance.saturating_sub(multisig.deposit));

		// Verify event
		System::assert_last_event(
			Event::MultisigCreated {
				multisig: pallet_multisig::Pallet::<Test>::multi_account_id(0),
				creator: 1,
				threshold,
			}
			.into(),
		);
	});
}

#[test]
fn create_multisig_fails_with_invalid_threshold() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 2, 3];

		// Threshold of 0
		assert_noop!(
			pallet_multisig::Pallet::<Test>::create_multisig(
				RuntimeOrigin::signed(1),
				owners.clone(),
				0
			),
			Error::<Test>::InvalidThreshold
		);

		// Threshold greater than number of owners
		assert_noop!(
			pallet_multisig::Pallet::<Test>::create_multisig(
				RuntimeOrigin::signed(1),
				owners.clone(),
				4
			),
			Error::<Test>::InvalidThreshold
		);
	});
}

#[test]
fn create_multisig_fails_with_duplicate_owners() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 1, 2]; // Duplicate owner 1

		assert_noop!(
			pallet_multisig::Pallet::<Test>::create_multisig(RuntimeOrigin::signed(1), owners, 2),
			Error::<Test>::AlreadyOwner
		);
	});
}

#[test]
fn create_multisig_fails_with_too_many_owners() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners: Vec<u64> = (1..=51).collect(); // 51 owners when max is 50

		assert_noop!(
			pallet_multisig::Pallet::<Test>::create_multisig(RuntimeOrigin::signed(1), owners, 26),
			Error::<Test>::TooManyOwners
		);
	});
}

#[test]
fn create_multisig_fails_with_empty_owners() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![];

		assert_noop!(
			pallet_multisig::Pallet::<Test>::create_multisig(RuntimeOrigin::signed(1), owners, 1),
			Error::<Test>::InvalidThreshold
		);
	});
}
