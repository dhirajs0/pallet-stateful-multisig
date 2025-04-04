use crate as pallet_multisig;
use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};

/// Tests for the revoke_confirmation extrinsic
/// This module tests the revocation of transaction confirmations by multisig owners,
/// including proper storage updates, event emissions, and various failure cases.
/// It verifies that confirmations can be revoked before transaction execution.

#[test]
fn revoke_confirmation_works_for_submitter() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 2, 3];
		let threshold = 2;
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

		// Create multisig and submit transaction
		assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
			RuntimeOrigin::signed(1),
			owners.clone(),
			threshold
		));

		let call =
			Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));
		assert_ok!(pallet_multisig::Pallet::<Test>::submit_transaction(
			RuntimeOrigin::signed(1),
			multisig,
			call
		));

		// Verify initial confirmation count
		assert_eq!(pallet_multisig::Pallet::<Test>::count_confirmations(&multisig, 0), 1);

		// Revoke confirmation
		assert_ok!(pallet_multisig::Pallet::<Test>::revoke_confirmation(
			RuntimeOrigin::signed(1),
			multisig,
			0
		));

		// Verify confirmation was removed
		assert_eq!(pallet_multisig::Pallet::<Test>::count_confirmations(&multisig, 0), 0);
		assert_eq!(Confirmations::<Test>::get(&multisig, (0, 1)), None);

		// Verify event
		System::assert_last_event(
			Event::TransactionUnsigned { multisig, transaction_id: 0, signer: 1 }.into(),
		);

		// Verify can confirm again after revoking
		assert_ok!(pallet_multisig::Pallet::<Test>::confirm_transaction(
			RuntimeOrigin::signed(1),
			multisig,
			0
		));
		assert_eq!(pallet_multisig::Pallet::<Test>::count_confirmations(&multisig, 0), 1);
	});
}

#[test]
fn revoke_confirmation_works_for_other_owner() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 2, 3];
		let threshold = 3;
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

		// Create multisig and submit transaction
		assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
			RuntimeOrigin::signed(1),
			owners.clone(),
			threshold
		));

		let call =
			Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));
		assert_ok!(pallet_multisig::Pallet::<Test>::submit_transaction(
			RuntimeOrigin::signed(1),
			multisig,
			call
		));

		// Add second confirmation
		assert_ok!(pallet_multisig::Pallet::<Test>::confirm_transaction(
			RuntimeOrigin::signed(2),
			multisig,
			0
		));

		// Verify initial confirmation count
		assert_eq!(pallet_multisig::Pallet::<Test>::count_confirmations(&multisig, 0), 2);

		// Revoke second confirmation
		assert_ok!(pallet_multisig::Pallet::<Test>::revoke_confirmation(
			RuntimeOrigin::signed(2),
			multisig,
			0
		));

		// Verify confirmation was removed
		assert_eq!(pallet_multisig::Pallet::<Test>::count_confirmations(&multisig, 0), 1);
		assert_eq!(Confirmations::<Test>::get(&multisig, (0, 2)), None);

		// Verify event
		System::assert_last_event(
			Event::TransactionUnsigned { multisig, transaction_id: 0, signer: 2 }.into(),
		);

		// Verify can confirm again after revoking
		assert_ok!(pallet_multisig::Pallet::<Test>::confirm_transaction(
			RuntimeOrigin::signed(2),
			multisig,
			0
		));
		assert_eq!(pallet_multisig::Pallet::<Test>::count_confirmations(&multisig, 0), 2);
	});
}

#[test]
fn revoke_confirmation_fails_when_not_confirmed() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 2, 3];
		let threshold = 2;
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

		// Create multisig and submit transaction
		assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
			RuntimeOrigin::signed(1),
			owners.clone(),
			threshold
		));

		let call =
			Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));
		assert_ok!(pallet_multisig::Pallet::<Test>::submit_transaction(
			RuntimeOrigin::signed(1),
			multisig,
			call
		));

		// Try to revoke non-existent confirmation
		assert_noop!(
			pallet_multisig::Pallet::<Test>::revoke_confirmation(
				RuntimeOrigin::signed(2),
				multisig,
				0
			),
			Error::<Test>::NotConfirmed
		);
	});
}

#[test]
fn revoke_confirmation_fails_when_already_executed() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 2, 3];
		let threshold = 2;
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

		// Create multisig and submit transaction
		assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
			RuntimeOrigin::signed(1),
			owners.clone(),
			threshold
		));

		let call =
			Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));
		assert_ok!(pallet_multisig::Pallet::<Test>::submit_transaction(
			RuntimeOrigin::signed(1),
			multisig,
			call
		));

		// Add second confirmation to trigger execution
		assert_ok!(pallet_multisig::Pallet::<Test>::confirm_transaction(
			RuntimeOrigin::signed(2),
			multisig,
			0
		));

		// Try to revoke confirmation after execution
		assert_noop!(
			pallet_multisig::Pallet::<Test>::revoke_confirmation(
				RuntimeOrigin::signed(1),
				multisig,
				0
			),
			Error::<Test>::AlreadyExecuted
		);
	});
}

#[test]
fn revoke_confirmation_fails_for_non_owner() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 2, 3];
		let threshold = 2;
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

		// Create multisig and submit transaction
		assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
			RuntimeOrigin::signed(1),
			owners.clone(),
			threshold
		));

		let call =
			Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));
		assert_ok!(pallet_multisig::Pallet::<Test>::submit_transaction(
			RuntimeOrigin::signed(1),
			multisig,
			call
		));

		// Try to revoke with non-owner
		assert_noop!(
			pallet_multisig::Pallet::<Test>::revoke_confirmation(
				RuntimeOrigin::signed(4),
				multisig,
				0
			),
			Error::<Test>::NotOwner
		);
	});
}

#[test]
fn revoke_confirmation_fails_for_nonexistent_transaction() {
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

		// Try to revoke non-existent transaction
		assert_noop!(
			pallet_multisig::Pallet::<Test>::revoke_confirmation(
				RuntimeOrigin::signed(1),
				multisig,
				0
			),
			Error::<Test>::TransactionNotFound
		);
	});
}
