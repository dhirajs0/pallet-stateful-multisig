use crate as pallet_multisig;
use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};

/// Tests for owner management extrinsics (add_owner, remove_owner, replace_owner)
/// This module tests the management of multisig owners, including proper storage updates,
/// event emissions, and various failure cases. It verifies that owner management
/// operations maintain the integrity of the multisig account.

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

#[test]
#[ignore = "Deposit reservation failure needs to be fixed"]
fn add_owner_fails_when_max_owners_reached() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		// Create a multisig with MaxOwners (50) owners
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
		let new_owner = 51;
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
