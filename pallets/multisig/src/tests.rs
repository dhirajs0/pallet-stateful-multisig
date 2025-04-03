use crate as pallet_multisig;
use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};

#[test]
fn create_multisig_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 2, 3];
		let threshold = 2;

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

#[ignore]
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
		System::assert_last_event(
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

// ignore this test for now
// #[ignore]
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

#[ignore]
#[test]
fn revoke_confirmation_works_for_other_owner() {
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

#[test]
fn add_owner_works() {
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

		// Add a new owner
		let new_owner = 4;
		assert_ok!(pallet_multisig::Pallet::<Test>::add_owner(
			RuntimeOrigin::signed(multisig),
			multisig,
			new_owner
		));

		// Verify the new owner is added to the owners list
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert!(multisig_data.owners.contains(&new_owner));

		// Verify the new owner is added to the MultisigOwners storage
		assert!(pallet_multisig::MultisigOwners::<Test>::contains_key(&multisig, &new_owner));

		// Verify event
		System::assert_last_event(Event::OwnerAdded { multisig, new_owner }.into());
	});
}

#[test]
fn add_owner_fails_when_called_by_non_multisig() {
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

		// Try to add a new owner from a non-multisig account
		let new_owner = 4;
		assert_noop!(
			pallet_multisig::Pallet::<Test>::add_owner(
				RuntimeOrigin::signed(1),
				multisig,
				new_owner
			),
			Error::<Test>::NotOwner
		);
	});
}

#[test]
fn add_owner_fails_when_owner_already_exists() {
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

		// Try to add an existing owner
		assert_noop!(
			pallet_multisig::Pallet::<Test>::add_owner(
				RuntimeOrigin::signed(multisig),
				multisig,
				1
			),
			Error::<Test>::AlreadyOwner
		);
	});
}

#[ignore] // TODO: fix
#[test]
fn add_owner_fails_when_max_owners_reached() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		// Create a multisig with MaxOwners (3) owners

		let owners: Vec<u64> = (1..=50).collect();
		let threshold = 2;
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

		// Create multisig
		assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
			RuntimeOrigin::signed(1),
			owners.clone(),
			threshold
		));

		// Try to add a new owner when at max capacity
		let new_owner = 4;
		assert_noop!(
			pallet_multisig::Pallet::<Test>::add_owner(
				RuntimeOrigin::signed(multisig),
				multisig,
				new_owner
			),
			Error::<Test>::TooManyOwners
		);
	});
}

#[test]
fn add_owner_fails_for_nonexistent_multisig() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);
		let new_owner = 4;

		// Try to add an owner to a non-existent multisig
		assert_noop!(
			pallet_multisig::Pallet::<Test>::add_owner(
				RuntimeOrigin::signed(multisig),
				multisig,
				new_owner
			),
			Error::<Test>::MultisigNotFound
		);
	});
}

#[test]
fn add_owner_through_multisig_transaction() {
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

		// Submit a transaction to add a new owner
		let new_owner = 4;
		let call = Box::new(RuntimeCall::Multisig(pallet_multisig::Call::add_owner {
			multisig,
			new_owner,
		}));

		// Submit the transaction
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

		// Verify the new owner is added to the owners list
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert!(multisig_data.owners.contains(&new_owner));

		// Verify the new owner is added to the MultisigOwners storage
		assert!(pallet_multisig::MultisigOwners::<Test>::contains_key(&multisig, &new_owner));
	});
}

#[test]
fn remove_owner_works() {
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

		// Remove an owner
		let old_owner = 3;
		assert_ok!(pallet_multisig::Pallet::<Test>::remove_owner(
			RuntimeOrigin::signed(multisig),
			multisig,
			old_owner
		));

		// Verify the owner is removed from the owners list
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert!(!multisig_data.owners.contains(&old_owner));

		// Verify the owner is removed from the MultisigOwners storage
		assert!(!pallet_multisig::MultisigOwners::<Test>::contains_key(&multisig, &old_owner));

		// Verify event
		System::assert_last_event(Event::OwnerRemoved { multisig, old_owner }.into());
	});
}

#[test]
fn remove_owner_fails_when_called_by_non_multisig() {
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

		// Try to remove an owner from a non-multisig account
		let old_owner = 3;
		assert_noop!(
			pallet_multisig::Pallet::<Test>::remove_owner(
				RuntimeOrigin::signed(1),
				multisig,
				old_owner
			),
			Error::<Test>::NotOwner
		);
	});
}

#[test]
fn remove_owner_fails_when_owner_does_not_exist() {
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

		// Try to remove a non-existent owner
		let non_existent_owner = 4;
		assert_noop!(
			pallet_multisig::Pallet::<Test>::remove_owner(
				RuntimeOrigin::signed(multisig),
				multisig,
				non_existent_owner
			),
			Error::<Test>::OwnerNotFound
		);
	});
}

#[test]
fn remove_owner_fails_for_nonexistent_multisig() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);
		let old_owner = 3;

		// Try to remove an owner from a non-existent multisig
		assert_noop!(
			pallet_multisig::Pallet::<Test>::remove_owner(
				RuntimeOrigin::signed(multisig),
				multisig,
				old_owner
			),
			Error::<Test>::MultisigNotFound
		);
	});
}

#[test]
fn remove_owner_adjusts_threshold_when_needed() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 2, 3];
		let threshold = 3; // Threshold equals owner count
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

		// Create multisig
		assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
			RuntimeOrigin::signed(1),
			owners.clone(),
			threshold
		));

		// Remove an owner
		let old_owner = 3;
		assert_ok!(pallet_multisig::Pallet::<Test>::remove_owner(
			RuntimeOrigin::signed(multisig),
			multisig,
			old_owner
		));

		// Verify the owner is removed
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert!(!multisig_data.owners.contains(&old_owner));

		// Verify the threshold is adjusted
		assert_eq!(multisig_data.required, 2); // Should be reduced to match new owner count

		// Verify events
		System::assert_has_event(Event::RequirementChanged { multisig, new_threshold: 2 }.into());
		System::assert_last_event(Event::OwnerRemoved { multisig, old_owner }.into());
	});
}

#[test]
fn remove_owner_through_multisig_transaction() {
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

		// Submit a transaction to remove an owner
		let old_owner = 3;
		let call = Box::new(RuntimeCall::Multisig(pallet_multisig::Call::remove_owner {
			multisig,
			old_owner,
		}));

		// Submit the transaction
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

		// Verify the owner is removed from the owners list
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert!(!multisig_data.owners.contains(&old_owner));

		// Verify the owner is removed from the MultisigOwners storage
		assert!(!pallet_multisig::MultisigOwners::<Test>::contains_key(&multisig, &old_owner));
	});
}

#[test]
fn remove_owner_maintains_threshold_when_not_needed() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 2, 3];
		let threshold = 2; // Threshold less than owner count
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

		// Create multisig
		assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
			RuntimeOrigin::signed(1),
			owners.clone(),
			threshold
		));

		// Remove an owner
		let old_owner = 3;
		assert_ok!(pallet_multisig::Pallet::<Test>::remove_owner(
			RuntimeOrigin::signed(multisig),
			multisig,
			old_owner
		));

		// Verify the owner is removed
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert!(!multisig_data.owners.contains(&old_owner));

		// Verify the threshold is not adjusted
		assert_eq!(multisig_data.required, threshold);

		// Verify only the OwnerRemoved event is emitted (no RequirementChanged)
		System::assert_last_event(Event::OwnerRemoved { multisig, old_owner }.into());
	});
}

#[test]
fn replace_owner_works() {
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

		// Replace an owner
		let old_owner = 3;
		let new_owner = 4;
		assert_ok!(pallet_multisig::Pallet::<Test>::replace_owner(
			RuntimeOrigin::signed(multisig),
			multisig,
			old_owner,
			new_owner
		));

		// Verify the owner is replaced in the owners list
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert!(!multisig_data.owners.contains(&old_owner));
		assert!(multisig_data.owners.contains(&new_owner));

		// Verify the MultisigOwners storage is updated correctly
		assert!(!pallet_multisig::MultisigOwners::<Test>::contains_key(&multisig, &old_owner));
		assert!(pallet_multisig::MultisigOwners::<Test>::contains_key(&multisig, &new_owner));

		// Verify events are emitted in the correct order
		System::assert_has_event(Event::OwnerRemoved { multisig, old_owner }.into());
		System::assert_last_event(Event::OwnerAdded { multisig, new_owner }.into());
	});
}

#[test]
fn replace_owner_fails_when_called_by_non_multisig() {
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

		// Try to replace an owner from a non-multisig account
		let old_owner = 3;
		let new_owner = 4;
		assert_noop!(
			pallet_multisig::Pallet::<Test>::replace_owner(
				RuntimeOrigin::signed(1),
				multisig,
				old_owner,
				new_owner
			),
			Error::<Test>::NotOwner
		);
	});
}

#[test]
fn replace_owner_fails_when_old_owner_does_not_exist() {
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

		// Try to replace a non-existent owner
		let non_existent_owner = 5;
		let new_owner = 4;
		assert_noop!(
			pallet_multisig::Pallet::<Test>::replace_owner(
				RuntimeOrigin::signed(multisig),
				multisig,
				non_existent_owner,
				new_owner
			),
			Error::<Test>::OwnerNotFound
		);
	});
}

#[test]
fn replace_owner_fails_when_new_owner_already_exists() {
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

		// Try to replace with an existing owner
		let old_owner = 3;
		let existing_owner = 1;
		assert_noop!(
			pallet_multisig::Pallet::<Test>::replace_owner(
				RuntimeOrigin::signed(multisig),
				multisig,
				old_owner,
				existing_owner
			),
			Error::<Test>::AlreadyOwner
		);
	});
}

#[test]
fn replace_owner_fails_for_nonexistent_multisig() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);
		let old_owner = 3;
		let new_owner = 4;

		// Try to replace an owner in a non-existent multisig
		assert_noop!(
			pallet_multisig::Pallet::<Test>::replace_owner(
				RuntimeOrigin::signed(multisig),
				multisig,
				old_owner,
				new_owner
			),
			Error::<Test>::MultisigNotFound
		);
	});
}

#[test]
fn replace_owner_through_multisig_transaction() {
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

		// Submit a transaction to replace an owner
		let old_owner = 3;
		let new_owner = 4;
		let call = Box::new(RuntimeCall::Multisig(pallet_multisig::Call::replace_owner {
			multisig,
			old_owner,
			new_owner,
		}));

		// Submit the transaction
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

		// Verify the owner is replaced in the owners list
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert!(!multisig_data.owners.contains(&old_owner));
		assert!(multisig_data.owners.contains(&new_owner));

		// Verify the MultisigOwners storage is updated correctly
		assert!(!pallet_multisig::MultisigOwners::<Test>::contains_key(&multisig, &old_owner));
		assert!(pallet_multisig::MultisigOwners::<Test>::contains_key(&multisig, &new_owner));
	});
}

#[test]
fn replace_owner_maintains_threshold() {
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

		// Replace an owner
		let old_owner = 3;
		let new_owner = 4;
		assert_ok!(pallet_multisig::Pallet::<Test>::replace_owner(
			RuntimeOrigin::signed(multisig),
			multisig,
			old_owner,
			new_owner
		));

		// Verify the threshold is maintained
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert_eq!(multisig_data.required, threshold);

		// Verify no RequirementChanged event is emitted
		System::assert_has_event(Event::OwnerRemoved { multisig, old_owner }.into());
		System::assert_last_event(Event::OwnerAdded { multisig, new_owner }.into());
	});
}

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

		// Change threshold to a new value
		let new_threshold = 3;
		assert_ok!(pallet_multisig::Pallet::<Test>::change_requirement(
			RuntimeOrigin::signed(multisig),
			multisig,
			new_threshold
		));

		// Verify the threshold is updated in storage
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert_eq!(multisig_data.required, new_threshold);

		// Verify event is emitted with correct parameters
		System::assert_last_event(Event::RequirementChanged { multisig, new_threshold }.into());
	});
}

#[test]
fn change_requirement_to_minimum_works() {
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

		// Change threshold to minimum (1)
		let new_threshold = 1;
		assert_ok!(pallet_multisig::Pallet::<Test>::change_requirement(
			RuntimeOrigin::signed(multisig),
			multisig,
			new_threshold
		));

		// Verify the threshold is updated in storage
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert_eq!(multisig_data.required, new_threshold);

		// Verify event is emitted with correct parameters
		System::assert_last_event(Event::RequirementChanged { multisig, new_threshold }.into());
	});
}

#[test]
fn change_requirement_to_maximum_works() {
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

		// Change threshold to maximum (owner count)
		let new_threshold = owners.len() as u32;
		assert_ok!(pallet_multisig::Pallet::<Test>::change_requirement(
			RuntimeOrigin::signed(multisig),
			multisig,
			new_threshold
		));

		// Verify the threshold is updated in storage
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert_eq!(multisig_data.required, new_threshold);

		// Verify event is emitted with correct parameters
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
fn change_requirement_fails_for_nonexistent_multisig() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);
		let new_threshold = 3;

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
fn change_requirement_fails_with_zero_threshold() {
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

		// Try to set threshold to 0
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
fn change_requirement_fails_with_threshold_greater_than_owners() {
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
		let new_threshold = owners.len() as u32 + 1;
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
fn change_requirement_through_multisig_transaction() {
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

		// Submit a transaction to change the threshold
		let new_threshold = 3;
		let call = Box::new(RuntimeCall::Multisig(pallet_multisig::Call::change_requirement {
			multisig,
			new_threshold,
		}));

		// Submit the transaction
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

		// Verify the threshold is updated in storage
		let multisig_data = pallet_multisig::Multisigs::<Test>::get(&multisig).unwrap();
		assert_eq!(multisig_data.required, new_threshold);
	});
}

#[test]
fn destroy_multisig_works() {
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

		// Destroy multisig as owner 1
		assert_ok!(pallet_multisig::Pallet::<Test>::destroy_multisig(
			RuntimeOrigin::signed(1),
			multisig
		));

		// Verify multisig was removed from storage
		assert!(Multisigs::<Test>::get(&multisig).is_none());

		// Verify owner mappings were removed
		for owner in owners.iter() {
			assert!(MultisigOwners::<Test>::get(&multisig, owner).is_none());
		}

		// Verify event
		System::assert_last_event(
			Event::MultisigDestroyed { multisig, approving_account: 1 }.into(),
		);
	});
}

#[test]
fn destroy_multisig_works_for_any_owner() {
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

		// Destroy multisig as owner 2
		assert_ok!(pallet_multisig::Pallet::<Test>::destroy_multisig(
			RuntimeOrigin::signed(2),
			multisig
		));

		// Verify multisig was removed from storage
		assert!(Multisigs::<Test>::get(&multisig).is_none());

		// Verify event
		System::assert_last_event(
			Event::MultisigDestroyed { multisig, approving_account: 2 }.into(),
		);
	});
}

#[test]
fn destroy_multisig_fails_for_non_owner() {
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

		// Try to destroy multisig as non-owner
		assert_noop!(
			pallet_multisig::Pallet::<Test>::destroy_multisig(RuntimeOrigin::signed(4), multisig),
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
			pallet_multisig::Pallet::<Test>::destroy_multisig(RuntimeOrigin::signed(1), multisig),
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

		// Verify all storage items were cleaned up
		assert!(Multisigs::<Test>::get(&multisig).is_none());
		assert!(Transactions::<Test>::get(&multisig, 0).is_none());
		assert!(Confirmations::<Test>::get(&multisig, (0, 1)).is_none());
		assert!(Confirmations::<Test>::get(&multisig, (0, 2)).is_none());
	});
}

#[test]
fn destroyed_multisig_cannot_be_used() {
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

		// Destroy multisig
		assert_ok!(pallet_multisig::Pallet::<Test>::destroy_multisig(
			RuntimeOrigin::signed(1),
			multisig
		));

		// Try to submit a transaction to destroyed multisig
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

		// Try to confirm a transaction on destroyed multisig
		assert_noop!(
			pallet_multisig::Pallet::<Test>::confirm_transaction(
				RuntimeOrigin::signed(1),
				multisig,
				0
			),
			Error::<Test>::MultisigNotFound
		);

		// Try to revoke a confirmation on destroyed multisig
		assert_noop!(
			pallet_multisig::Pallet::<Test>::revoke_confirmation(
				RuntimeOrigin::signed(1),
				multisig,
				0
			),
			Error::<Test>::MultisigNotFound
		);
	});
}

#[test]
fn destroy_multisig_with_pending_transactions() {
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

		// Submit multiple transactions
		for i in 0..3 {
			let call =
				Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![i as u8] }));
			assert_ok!(pallet_multisig::Pallet::<Test>::submit_transaction(
				RuntimeOrigin::signed(1),
				multisig,
				call
			));
		}

		// Confirm some transactions
		assert_ok!(pallet_multisig::Pallet::<Test>::confirm_transaction(
			RuntimeOrigin::signed(2),
			multisig,
			0
		));

		// Destroy the multisig
		assert_ok!(pallet_multisig::Pallet::<Test>::destroy_multisig(
			RuntimeOrigin::signed(1),
			multisig
		));

		// Verify all storage is cleaned up
		assert!(Multisigs::<Test>::get(&multisig).is_none());
		assert!(MultisigOwners::<Test>::iter_prefix(&multisig).next().is_none());
		assert!(Transactions::<Test>::iter_prefix(&multisig).next().is_none());
		assert!(Confirmations::<Test>::iter_prefix(&multisig).next().is_none());
	});
}

#[test]
fn destroy_multisig_verifies_storage_cleanup() {
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

		// Submit and confirm transactions
		for i in 0..3 {
			let call =
				Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![i as u8] }));
			assert_ok!(pallet_multisig::Pallet::<Test>::submit_transaction(
				RuntimeOrigin::signed(1),
				multisig,
				call
			));
			assert_ok!(pallet_multisig::Pallet::<Test>::confirm_transaction(
				RuntimeOrigin::signed(2),
				multisig,
				i
			));
		}

		// Verify storage exists before destruction
		assert!(Multisigs::<Test>::get(&multisig).is_some());
		assert_eq!(MultisigOwners::<Test>::iter_prefix(&multisig).count(), 3);
		assert_eq!(Transactions::<Test>::iter_prefix(&multisig).count(), 3);
		assert_eq!(Confirmations::<Test>::iter_prefix(&multisig).count(), 6); // 3 transactions * 2 confirmations each

		// Destroy the multisig
		assert_ok!(pallet_multisig::Pallet::<Test>::destroy_multisig(
			RuntimeOrigin::signed(1),
			multisig
		));

		// Verify all storage is cleaned up
		assert!(Multisigs::<Test>::get(&multisig).is_none());
		assert!(MultisigOwners::<Test>::iter_prefix(&multisig).next().is_none());
		assert!(Transactions::<Test>::iter_prefix(&multisig).next().is_none());
		assert!(Confirmations::<Test>::iter_prefix(&multisig).next().is_none());
	});
}

#[test]
fn destroyed_multisig_usage_fails() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let owners = vec![1, 2, 3];
		let threshold = 2;
		let multisig = pallet_multisig::Pallet::<Test>::multi_account_id(0);

		// Create and destroy multisig
		assert_ok!(pallet_multisig::Pallet::<Test>::create_multisig(
			RuntimeOrigin::signed(1),
			owners.clone(),
			threshold
		));
		assert_ok!(pallet_multisig::Pallet::<Test>::destroy_multisig(
			RuntimeOrigin::signed(1),
			multisig
		));

		// Try to submit transaction
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

		// Try to confirm transaction
		assert_noop!(
			pallet_multisig::Pallet::<Test>::confirm_transaction(
				RuntimeOrigin::signed(1),
				multisig,
				0
			),
			Error::<Test>::MultisigNotFound
		);

		// Try to revoke confirmation
		assert_noop!(
			pallet_multisig::Pallet::<Test>::revoke_confirmation(
				RuntimeOrigin::signed(1),
				multisig,
				0
			),
			Error::<Test>::MultisigNotFound
		);

		// Try to add owner
		assert_noop!(
			pallet_multisig::Pallet::<Test>::add_owner(
				RuntimeOrigin::signed(multisig),
				multisig,
				4
			),
			Error::<Test>::MultisigNotFound
		);

		// Try to remove owner
		assert_noop!(
			pallet_multisig::Pallet::<Test>::remove_owner(
				RuntimeOrigin::signed(multisig),
				multisig,
				1
			),
			Error::<Test>::MultisigNotFound
		);

		// Try to replace owner
		assert_noop!(
			pallet_multisig::Pallet::<Test>::replace_owner(
				RuntimeOrigin::signed(multisig),
				multisig,
				1,
				4
			),
			Error::<Test>::MultisigNotFound
		);

		// Try to change requirement
		assert_noop!(
			pallet_multisig::Pallet::<Test>::change_requirement(
				RuntimeOrigin::signed(multisig),
				multisig,
				1
			),
			Error::<Test>::MultisigNotFound
		);
	});
}

// TODO: test a full transaction lifecycle
