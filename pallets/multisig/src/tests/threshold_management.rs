use crate as pallet_multisig;
use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};

/// Tests for threshold management extrinsics (change_requirement)
/// This module tests the management of multisig thresholds, including proper storage updates,
/// event emissions, and various failure cases. It verifies that threshold changes maintain
/// the integrity of the multisig account and respect the minimum and maximum threshold values.

#[test]
fn change_requirement_works() {
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

		// Change threshold
		let new_threshold = 3;
		assert_ok!(pallet_multisig::Pallet::<Test>::change_requirement(
			RuntimeOrigin::signed(multisig),
			multisig,
			new_threshold
		));

		// Verify the threshold is updated
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert_eq!(multisig_data.required, new_threshold);

		// Verify event
		System::assert_last_event(Event::RequirementChanged { multisig, new_threshold }.into());
	});
}

#[test]
fn change_requirement_fails_when_called_by_non_multisig() {
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

		// Try to change threshold from a non-multisig account
		let new_threshold = 3;
		assert_noop!(
			pallet_multisig::Pallet::<Test>::change_requirement(
				RuntimeOrigin::signed(1),
				multisig,
				new_threshold
			),
			Error::<Test>::NotOwner
		);
	});
}

#[test]
fn change_requirement_fails_when_threshold_is_zero() {
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

		// Try to set threshold to zero
		let new_threshold = 0;
		assert_noop!(
			pallet_multisig::Pallet::<Test>::change_requirement(
				RuntimeOrigin::signed(multisig),
				multisig,
				new_threshold
			),
			Error::<Test>::InvalidThreshold
		);
	});
}

#[test]
fn change_requirement_fails_when_threshold_exceeds_owner_count() {
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

		// Try to set threshold greater than owner count
		let new_threshold = 4;
		assert_noop!(
			pallet_multisig::Pallet::<Test>::change_requirement(
				RuntimeOrigin::signed(multisig),
				multisig,
				new_threshold
			),
			Error::<Test>::InvalidThreshold
		);
	});
}

#[test]
fn change_requirement_fails_for_nonexistent_multisig() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);
		let new_threshold = 2;

		// Try to change threshold for a non-existent multisig
		assert_noop!(
			pallet_multisig::Pallet::<Test>::change_requirement(
				RuntimeOrigin::signed(multisig),
				multisig,
				new_threshold
			),
			Error::<Test>::MultisigNotFound
		);
	});
}

#[test]
fn change_requirement_succeeds_when_threshold_equals_owner_count() {
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

		// Set threshold equal to owner count
		let new_threshold = 3;
		assert_ok!(pallet_multisig::Pallet::<Test>::change_requirement(
			RuntimeOrigin::signed(multisig),
			multisig,
			new_threshold
		));

		// Verify the threshold is updated
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert_eq!(multisig_data.required, new_threshold);

		// Verify event
		System::assert_last_event(Event::RequirementChanged { multisig, new_threshold }.into());
	});
}

#[test]
fn change_requirement_succeeds_when_threshold_is_one() {
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

		// Set threshold to one
		let new_threshold = 1;
		assert_ok!(pallet_multisig::Pallet::<Test>::change_requirement(
			RuntimeOrigin::signed(multisig),
			multisig,
			new_threshold
		));

		// Verify the threshold is updated
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert_eq!(multisig_data.required, new_threshold);

		// Verify event
		System::assert_last_event(Event::RequirementChanged { multisig, new_threshold }.into());
	});
}
