use crate as pallet_multisig;
use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};

/// Tests for the submit_transaction extrinsic
/// This module tests the submission of transactions to multisig accounts,
/// including proper storage updates, event emissions, and various failure cases.

#[test]
fn submit_transaction_works() {
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

		// Verify transaction was stored
		let transaction = Transactions::<Test>::get(&multisig, 0).unwrap();
		assert_eq!(transaction.executed, false);

		// Verify transaction count was incremented
		let multisig_data = Multisigs::<Test>::get(&multisig).unwrap();
		assert_eq!(multisig_data.transaction_count, 1);

		// Verify confirmation was recorded
		assert_eq!(Confirmations::<Test>::get(&multisig, (0, 1)), Some(true));

		// Verify events
		System::assert_has_event(
			Event::TransactionSubmitted { multisig, transaction_id: 0, submitter: 1 }.into(),
		);
		System::assert_has_event(
			Event::TransactionSigned { multisig, transaction_id: 0, signer: 1 }.into(),
		);
	});
}

#[test]
fn submit_transaction_fails_for_non_owner() {
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

		// Try to submit transaction as non-owner
		let call =
			Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));
		assert_noop!(
			pallet_multisig::Pallet::<Test>::submit_transaction(
				RuntimeOrigin::signed(4),
				multisig,
				call
			),
			Error::<Test>::NotOwner
		);
	});
}

#[test]
fn submit_transaction_fails_for_nonexistent_multisig() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

		// Try to submit transaction to non-existent multisig
		let call =
			Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));
		assert_noop!(
			pallet_multisig::Pallet::<Test>::submit_transaction(
				RuntimeOrigin::signed(1),
				multisig,
				call
			),
			Error::<Test>::MultisigNotFound
		);
	});
}
