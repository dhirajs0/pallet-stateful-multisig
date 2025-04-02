use crate as pallet_multisig;
use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};

#[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {
		// Go past genesis block so events get deposited
		System::set_block_number(1);
		// Dispatch a signed extrinsic.
		assert_ok!(pallet_multisig::Pallet::<Test>::do_something(RuntimeOrigin::signed(1), 42));
		// Read pallet storage and assert an expected result.
		assert_eq!(Something::<Test>::get(), Some(42));
		// Assert that the correct event was deposited
		System::assert_last_event(Event::SomethingStored { something: 42, who: 1 }.into());
	});
}

#[test]
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(
			pallet_multisig::Pallet::<Test>::cause_error(RuntimeOrigin::signed(1)),
			Error::<Test>::NoneValue
		);
	});
}

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
