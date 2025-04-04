use crate as pallet_multisig;
use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};

/// Tests for the confirm_transaction extrinsic
/// This module tests the confirmation of transactions by multisig owners,
/// including proper storage updates, event emissions, and various failure cases.
/// It also verifies the execution of transactions when the threshold is reached.

#[test]
fn confirm_transaction_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 2, 3];
		let threshold = 2;
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

		// Create multisig first
		assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
			RuntimeOrigin::signed(1),
			owners.clone(),
			threshold
		));

		// Submit a transaction
		let call =
			Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));
		assert_ok!(pallet_multisig::Pallet::<Test>::submit_transaction(
			RuntimeOrigin::signed(1),
			multisig,
			call
		));

		// First confirmation (already done by submitter)
		assert_eq!(pallet_multisig::Pallet::<Test>::count_confirmations(&multisig, 0), 1);

		// Second confirmation
		assert_ok!(pallet_multisig::Pallet::<Test>::confirm_transaction(
			RuntimeOrigin::signed(2),
			multisig,
			0
		));

		// Verify confirmation count
		assert_eq!(pallet_multisig::Pallet::<Test>::count_confirmations(&multisig, 0), 2);

		// Verify confirmation was recorded
		assert_eq!(Confirmations::<Test>::get(&multisig, (0, 2)), Some(true));

		// Verify event
		System::assert_has_event(
			Event::TransactionSigned { multisig, transaction_id: 0, signer: 2 }.into(),
		);
	});
}

#[test]
fn confirm_transaction_fails_when_already_confirmed() {
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

		// Try to confirm again with the same signer
		assert_noop!(
			pallet_multisig::Pallet::<Test>::confirm_transaction(
				RuntimeOrigin::signed(1),
				multisig,
				0
			),
			Error::<Test>::AlreadyConfirmed
		);
	});
}

#[test]
fn confirm_transaction_fails_for_non_owner() {
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

		// Try to confirm with non-owner
		assert_noop!(
			pallet_multisig::Pallet::<Test>::confirm_transaction(
				RuntimeOrigin::signed(4),
				multisig,
				0
			),
			Error::<Test>::NotOwner
		);
	});
}

#[test]
fn confirm_transaction_fails_for_nonexistent_transaction() {
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

		// Try to confirm non-existent transaction
		assert_noop!(
			pallet_multisig::Pallet::<Test>::confirm_transaction(
				RuntimeOrigin::signed(1),
				multisig,
				0
			),
			Error::<Test>::TransactionNotFound
		);
	});
}

#[test]
fn confirm_transaction_executes_at_threshold() {
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

		// Submit transaction
		let call =
			Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));
		assert_ok!(pallet_multisig::Pallet::<Test>::submit_transaction(
			RuntimeOrigin::signed(1),
			multisig,
			call
		));

		// First confirmation (already done by submitter)
		assert_eq!(pallet_multisig::Pallet::<Test>::count_confirmations(&multisig, 0), 1);

		// Second confirmation (should trigger execution)
		assert_ok!(pallet_multisig::Pallet::<Test>::confirm_transaction(
			RuntimeOrigin::signed(2),
			multisig,
			0
		));

		// Verify transaction is marked as executed
		let transaction = Transactions::<Test>::get(&multisig, 0).unwrap();
		assert!(transaction.executed);

		// Verify confirmation count
		assert_eq!(pallet_multisig::Pallet::<Test>::count_confirmations(&multisig, 0), 2);
	});
}

#[test]
fn confirm_transaction_fails_when_already_executed() {
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

		// First confirmation (already done by submitter)
		assert_eq!(pallet_multisig::Pallet::<Test>::count_confirmations(&multisig, 0), 1);

		// Second confirmation (triggers execution)
		assert_ok!(pallet_multisig::Pallet::<Test>::confirm_transaction(
			RuntimeOrigin::signed(2),
			multisig,
			0
		));

		// Try to confirm after execution
		assert_noop!(
			pallet_multisig::Pallet::<Test>::confirm_transaction(
				RuntimeOrigin::signed(3),
				multisig,
				0
			),
			Error::<Test>::AlreadyExecuted
		);
	});
}
