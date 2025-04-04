//! Benchmarking setup for pallet-multisig
//!
//! This module contains the benchmarking logic for the multisig pallet,
//! which measures the execution time of extrinsics to determine appropriate weights.
//!
//! Benchmarks are critical for properly calculating transaction fees and execution time
//! in a Substrate-based blockchain.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as Multisig;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
// Add these imports to fix the errors
use frame_support::traits::{
	fungible::{Inspect, Mutate},
	Get,
};
use sp_runtime::Saturating;
use sp_std::prelude::*;

/// Helper function to fund an account with the specified amount.
///
/// Ensures that test accounts have sufficient balance for any operations.
fn fund_account<T: Config>(account: &T::AccountId, amount: BalanceOf<T>) {
	// Ensure the account has enough balance for the benchmark operations
	let min_balance = T::NativeBalance::minimum_balance();
	let total_amount = amount.saturating_add(min_balance.saturating_mul(100u32.into()));

	// Use the Balances pallet functionality to set the balance
	T::NativeBalance::set_balance(account, total_amount);
}

/// Helper function to create a multisig wallet for benchmark tests.
///
/// Creates a multisig with the specified number of owners and threshold,
/// then verifies the multisig was created correctly.
fn create_multisig_for_benchmark<T: Config>(
	num_owners: u32,
	threshold: u32,
) -> (T::AccountId, T::AccountId, Vec<T::AccountId>) {
	// Create the creator account
	let creator: T::AccountId = account("creator", 0, 0);

	// Calculate deposit and fund the account
	let deposit = Multisig::<T>::calculate_deposit(num_owners).unwrap();
	fund_account::<T>(&creator, deposit.saturating_mul(2u32.into()));

	// Create owner accounts
	let mut owners: Vec<T::AccountId> = Vec::new();
	for i in 0..num_owners {
		let owner = account("owner", i, 0);
		owners.push(owner);
	}

	// Create the multisig
	assert!(Multisig::<T>::create_multisig(
		RawOrigin::Signed(creator.clone()).into(),
		owners.clone(),
		threshold
	)
	.is_ok());

	// Get the multisig account ID
	let seed = LastMultisigSeed::<T>::get() - 1;
	let multisig_account = Multisig::<T>::multi_account_id(seed);

	// Verify the multisig was created correctly
	assert!(Multisigs::<T>::contains_key(&multisig_account));

	(multisig_account, creator, owners)
}

/// Helper function to create a simple call for testing transactions.
///
/// Returns a call that has minimal execution time so we can measure
/// just the multisig operation itself.
fn create_benchmark_call<T: Config>() -> Box<<T as frame_system::Config>::RuntimeCall> {
	// The remark call is a minimal call that does almost nothing
	let call = frame_system::Call::<T>::remark { remark: vec![0; 10] };
	Box::new(call.into())
}

/// Helper to submit a transaction to a multisig.
///
/// Creates and submits a transaction, then returns the transaction ID.
fn submit_transaction_for_benchmark<T: Config>(
	multisig_account: &T::AccountId,
	submitter: &T::AccountId,
	call: Box<<T as frame_system::Config>::RuntimeCall>,
) -> u32 {
	// Get the initial transaction count
	let initial_count = Multisigs::<T>::get(multisig_account).unwrap().transaction_count;

	// Submit the transaction
	assert!(Multisig::<T>::submit_transaction(
		RawOrigin::Signed(submitter.clone()).into(),
		multisig_account.clone(),
		call
	)
	.is_ok());

	// Return the transaction ID (which is the previous transaction count)
	initial_count
}

#[benchmarks]
mod benchmarks {
	use super::*;

	/// Benchmark for `create_multisig` extrinsic.
	///
	/// This benchmark measures the performance of creating a multisig wallet with
	/// varying numbers of owners and different threshold values.
	///
	/// Parameters:
	/// - o: Number of owners in the multisig (varies from 1 to MaxOwners)
	/// - t: Threshold parameter (percentage of owners required for approval)
	#[benchmark]
	fn create_multisig(
		o: Linear<1, { T::MaxOwners::get() }>, // Number of owners, varies from 1 to MaxOwners
		t: Linear<1, 100>,                     // Threshold as percentage of owners (1-100%)
	) {
		// SETUP PHASE: Prepare all necessary preconditions for the extrinsic

		// 1. Create the caller account and ensure it has sufficient funds
		let caller: T::AccountId = account("caller", 0, 0); // Using benchmarking's built-in account function

		// 2. Calculate how much deposit will be needed for the multisig
		let deposit = Multisig::<T>::calculate_deposit(o).unwrap();

		// 3. Fund the caller account with enough balance to cover the deposit
		fund_account::<T>(&caller, deposit.saturating_mul(2u32.into()));

		// 4. Generate the specified number of owner accounts
		let mut owners: Vec<T::AccountId> = Vec::new();
		for i in 0..o {
			let owner = account("owner", i + 1, 0); // Use built-in account function
			owners.push(owner);
		}

		// 5. Calculate the actual threshold value based on percentage parameter
		let threshold = core::cmp::max(1, (o as u128 * t as u128 / 100) as u32);
		let threshold = core::cmp::min(threshold, o); // Ensure threshold <= owners count

		// Verify our setup is correct
		assert!(threshold > 0, "Threshold must be positive");
		assert!(threshold <= o, "Threshold cannot exceed owner count");
		assert!(owners.len() == o as usize, "Owner count must match the parameter");

		// EXECUTION PHASE: Execute the extrinsic being benchmarked
		#[extrinsic_call]
		create_multisig(RawOrigin::Signed(caller), owners, threshold);

		// VERIFICATION PHASE: Verify the extrinsic executed successfully

		// The multisig account ID is derived from the seed which equals the previous
		// value of LastMultisigSeed storage item (which is 0 for the first multisig)
		let seed = LastMultisigSeed::<T>::get() - 1;
		let multisig_account = Multisig::<T>::multi_account_id(seed);

		// Check that the multisig was properly created
		assert!(Multisigs::<T>::contains_key(&multisig_account), "Multisig not created");

		// Check the multisig configuration
		let multisig_data = Multisigs::<T>::get(&multisig_account).unwrap();
		assert_eq!(multisig_data.required, threshold, "Incorrect threshold");
		assert_eq!(multisig_data.owners.len(), o as usize, "Incorrect owner count");
	}

	/// Benchmark for `submit_transaction` extrinsic.
	///
	/// This benchmark measures the performance of submitting a transaction to a multisig
	/// with varying numbers of existing owners.
	///
	/// Parameters:
	/// - o: Number of owners in the multisig
	#[benchmark]
	fn submit_transaction(o: Linear<2, { T::MaxOwners::get() }>, // Number of owners
	) {
		// SETUP PHASE: Create a multisig with specified owners

		// 1. Create a multisig with o owners and threshold of 2
		let (multisig_account, _, owners) = create_multisig_for_benchmark::<T>(o, 2);

		// 2. Select the first owner as the transaction submitter
		let submitter = owners[0].clone();

		// 3. Create a simple call to benchmark
		let call = create_benchmark_call::<T>();

		// EXECUTION PHASE: Execute the extrinsic being benchmarked
		#[extrinsic_call]
		submit_transaction(RawOrigin::Signed(submitter), multisig_account.clone(), call);

		// VERIFICATION PHASE: Verify the transaction was submitted

		// 1. Check that the transaction count was incremented
		let multisig_data = Multisigs::<T>::get(&multisig_account).unwrap();
		assert_eq!(multisig_data.transaction_count, 1, "Transaction count not incremented");

		// 2. Check that the transaction exists
		assert!(Transactions::<T>::contains_key(&multisig_account, 0), "Transaction not created");

		// 3. Check that the submitter's confirmation was recorded
		assert!(
			Confirmations::<T>::contains_key(&multisig_account, (0, owners[0].clone())),
			"Submitter confirmation not recorded"
		);
	}

	/// Benchmark for `confirm_transaction` extrinsic.
	///
	/// This benchmark measures the performance of confirming a transaction
	/// in a multisig with varying numbers of owners.
	///
	/// Parameters:
	/// - o: Number of owners in the multisig
	/// - c: Number of confirmations already made (excluding the new one)
	#[benchmark]
	fn confirm_transaction(
		o: Linear<3, { T::MaxOwners::get() }>, // Number of owners
		c: Linear<0, 100>,                     // Percentage of required confirmations already made
	) {
		// SETUP PHASE: Create a multisig and a transaction

		// 1. Create a multisig with o owners and threshold that's at least 2
		// (We need at least 2 for confirmation to make sense - one to submit, one to confirm)
		let threshold = core::cmp::max(2, o / 2);
		let (multisig_account, _, owners) = create_multisig_for_benchmark::<T>(o, threshold);

		// 2. First owner submits a transaction
		let submitter = owners[0].clone();
		let call = create_benchmark_call::<T>();
		let transaction_id =
			submit_transaction_for_benchmark::<T>(&multisig_account, &submitter, call);

		// 3. Calculate how many confirmations to add based on c parameter
		// c is a percentage of threshold-1 (excluding the submitter who already confirmed)
		let max_confirmations = threshold - 1; // How many more needed for execution
		let confirmations_to_add = (max_confirmations as u128 * c as u128 / 100) as u32;

		// 4. Add the calculated number of confirmations, but ensure we don't reach threshold
		for i in 1..=confirmations_to_add {
			if i < max_confirmations {
				// Only add confirmations if we won't reach threshold
				assert!(Multisig::<T>::confirm_transaction(
					RawOrigin::Signed(owners[i as usize].clone()).into(),
					multisig_account.clone(),
					transaction_id
				)
				.is_ok());
			}
		}

		// 5. Select the next unconfirmed owner as the confirmer
		let confirmer = owners[(confirmations_to_add + 1) as usize].clone();

		// EXECUTION PHASE: Execute the extrinsic being benchmarked
		#[extrinsic_call]
		confirm_transaction(
			RawOrigin::Signed(confirmer.clone()),
			multisig_account.clone(),
			transaction_id,
		);

		// VERIFICATION PHASE

		// Check that the confirmation was recorded
		assert!(
			Confirmations::<T>::contains_key(&multisig_account, (transaction_id, confirmer)),
			"Confirmation not recorded"
		);

		// Check if the transaction was executed (if we reached the threshold)
		let txn = Transactions::<T>::get(&multisig_account, transaction_id).unwrap();
		let current_confirmations = confirmations_to_add + 2; // +1 for original submitter, +1 for this confirmation
		assert_eq!(
			txn.executed,
			current_confirmations >= threshold,
			"Transaction execution status incorrect"
		);
	}

	/// Benchmark for `revoke_confirmation` extrinsic.
	///
	/// This benchmark measures the performance of revoking a previously
	/// made confirmation on a multisig transaction.
	///
	/// Parameters:
	/// - o: Number of owners in the multisig
	#[benchmark]
	fn revoke_confirmation(o: Linear<2, { T::MaxOwners::get() }>, // Number of owners
	) {
		// SETUP PHASE: Create a multisig and a confirmed transaction

		// 1. Create a multisig with o owners and threshold of minimum 2
		let threshold = core::cmp::max(2, o / 2);
		let (multisig_account, _, owners) = create_multisig_for_benchmark::<T>(o, threshold);

		// 2. First owner submits a transaction (which automatically confirms it)
		let submitter = owners[0].clone();
		let call = create_benchmark_call::<T>();
		let transaction_id =
			submit_transaction_for_benchmark::<T>(&multisig_account, &submitter, call);

		// EXECUTION PHASE: Execute the extrinsic being benchmarked
		#[extrinsic_call]
		revoke_confirmation(
			RawOrigin::Signed(submitter.clone()),
			multisig_account.clone(),
			transaction_id,
		);

		// VERIFICATION PHASE

		// Check that the confirmation was removed
		assert!(
			!Confirmations::<T>::contains_key(&multisig_account, (transaction_id, submitter)),
			"Confirmation not revoked"
		);
	}

	/// Benchmark for `add_owner` extrinsic.
	///
	/// This benchmark measures the performance of adding a new owner
	/// to an existing multisig wallet.
	///
	/// Parameters:
	/// - o: Number of existing owners in the multisig
	#[benchmark]
	fn add_owner(
		o: Linear<1, { T::MaxOwners::get() - 1 }>, /* Number of initial owners (leaving room for
		                                            * 1 more) */
	) {
		// SETUP PHASE: Create a multisig and prepare for add_owner

		// 1. Create a multisig with o owners
		let threshold = core::cmp::max(1, o / 2);
		let (multisig_account, _, _) = create_multisig_for_benchmark::<T>(o, threshold);

		// 2. Create the new owner account
		let new_owner: T::AccountId = account("new_owner", 0, 0);

		// 3. Create and submit a transaction to add the new owner
		// This extrinsic must be called by the multisig itself
		// let call = crate::Call::<T>::add_owner {
		//     multisig: multisig_account.clone(),
		//     new_owner: new_owner.clone()
		// }.into();

		// EXECUTION PHASE: Execute the extrinsic being benchmarked
		// Note: This is a bit different since add_owner must come from the multisig account itself
		#[extrinsic_call]
		add_owner(
			RawOrigin::Signed(multisig_account.clone()),
			multisig_account.clone(),
			new_owner.clone(),
		);

		// VERIFICATION PHASE

		// Check that the new owner was added
		assert!(
			MultisigOwners::<T>::contains_key(&multisig_account, &new_owner),
			"New owner not added"
		);

		// Check that the owners list was updated
		let multisig_data = Multisigs::<T>::get(&multisig_account).unwrap();
		assert_eq!(multisig_data.owners.len(), (o + 1) as usize, "Owner count not incremented");
	}

	/// Benchmark for `remove_owner` extrinsic.
	///
	/// This benchmark measures the performance of removing an owner
	/// from an existing multisig wallet.
	///
	/// Parameters:
	/// - o: Number of existing owners in the multisig
	#[benchmark]
	fn remove_owner(
		o: Linear<2, { T::MaxOwners::get() }>, // Need at least 2 owners to remove one
	) {
		// SETUP PHASE: Create a multisig and prepare for remove_owner

		// 1. Create a multisig with o owners
		let threshold = core::cmp::max(1, o / 2);
		let (multisig_account, _, owners) = create_multisig_for_benchmark::<T>(o, threshold);

		// 2. Select the last owner to remove
		let owner_to_remove = owners.last().unwrap().clone();

		// EXECUTION PHASE: Execute the extrinsic being benchmarked
		#[extrinsic_call]
		remove_owner(
			RawOrigin::Signed(multisig_account.clone()),
			multisig_account.clone(),
			owner_to_remove.clone(),
		);

		// VERIFICATION PHASE

		// Check that the owner was removed
		assert!(
			!MultisigOwners::<T>::contains_key(&multisig_account, &owner_to_remove),
			"Owner not removed"
		);

		// Check that the owners list was updated
		let multisig_data = Multisigs::<T>::get(&multisig_account).unwrap();
		assert_eq!(multisig_data.owners.len(), (o - 1) as usize, "Owner count not decremented");
	}

	/// Benchmark for `replace_owner` extrinsic.
	///
	/// This benchmark measures the performance of replacing an owner
	/// in an existing multisig wallet with a new owner.
	///
	/// Parameters:
	/// - o: Number of existing owners in the multisig
	#[benchmark]
	fn replace_owner(o: Linear<2, { T::MaxOwners::get() }>, // Need at least 2 owners
	) {
		// SETUP PHASE: Create a multisig and prepare for replace_owner

		// 1. Create a multisig with o owners
		let threshold = core::cmp::max(1, o / 2);
		let (multisig_account, _, owners) = create_multisig_for_benchmark::<T>(o, threshold);

		// 2. Select an owner to replace (the last one)
		let old_owner = owners.last().unwrap().clone();

		// 3. Create a new owner account
		let new_owner: T::AccountId = account("new_owner", 0, 0);

		// EXECUTION PHASE: Execute the extrinsic being benchmarked
		#[extrinsic_call]
		replace_owner(
			RawOrigin::Signed(multisig_account.clone()),
			multisig_account.clone(),
			old_owner.clone(),
			new_owner.clone(),
		);

		// VERIFICATION PHASE

		// Check that the old owner was removed
		assert!(
			!MultisigOwners::<T>::contains_key(&multisig_account, &old_owner),
			"Old owner not removed"
		);

		// Check that the new owner was added
		assert!(
			MultisigOwners::<T>::contains_key(&multisig_account, &new_owner),
			"New owner not added"
		);

		// Check that the total number of owners didn't change
		let multisig_data = Multisigs::<T>::get(&multisig_account).unwrap();
		assert_eq!(multisig_data.owners.len(), o as usize, "Owner count changed incorrectly");
	}

	/// Benchmark for `change_requirement` extrinsic.
	///
	/// This benchmark measures the performance of changing the threshold
	/// requirement in an existing multisig wallet.
	///
	/// Parameters:
	/// - o: Number of owners in the multisig
	/// - t: New threshold as percentage of owners
	#[benchmark]
	fn change_requirement(
		o: Linear<2, { T::MaxOwners::get() }>, // Number of owners
		t: Linear<1, 100>,                     // New threshold as percentage of owners
	) {
		// SETUP PHASE: Create a multisig with a specific threshold

		// 1. Create a multisig with o owners and initial threshold of 1
		let (multisig_account, _, _) = create_multisig_for_benchmark::<T>(o, 1);

		// 2. Calculate the new threshold based on percentage
		let new_threshold = core::cmp::max(1, (o as u128 * t as u128 / 100) as u32);
		let new_threshold = core::cmp::min(new_threshold, o); // Ensure it's valid

		// EXECUTION PHASE: Execute the extrinsic being benchmarked
		#[extrinsic_call]
		change_requirement(
			RawOrigin::Signed(multisig_account.clone()),
			multisig_account.clone(),
			new_threshold,
		);

		// VERIFICATION PHASE

		// Check that the threshold was updated
		let multisig_data = Multisigs::<T>::get(&multisig_account).unwrap();
		assert_eq!(multisig_data.required, new_threshold, "Threshold not updated correctly");
	}

	/// Benchmark for `destroy_multisig` extrinsic.
	///
	/// This benchmark measures the performance of completely destroying
	/// a multisig wallet, including cleaning up all related storage.
	///
	/// Parameters:
	/// - o: Number of owners in the multisig
	/// - t: Number of transactions in the multisig
	#[benchmark]
	fn destroy_multisig(
		o: Linear<1, { T::MaxOwners::get() }>, // Number of owners
		t: Linear<0, 100>,                     // Number of transactions
	) {
		// SETUP PHASE: Create a multisig with transactions

		// 1. Create a multisig with o owners
		let threshold = core::cmp::max(1, o / 2);
		let (multisig_account, _, owners) = create_multisig_for_benchmark::<T>(o, threshold);

		// 2. Create t transactions
		let submitter = owners[0].clone();
		for _ in 0..t {
			let call = create_benchmark_call::<T>();
			let _ = submit_transaction_for_benchmark::<T>(&multisig_account, &submitter, call);
		}

		// EXECUTION PHASE: Execute the extrinsic being benchmarked
		#[extrinsic_call]
		destroy_multisig(RawOrigin::Signed(submitter.clone()), multisig_account.clone());

		// VERIFICATION PHASE

		// Check that the multisig was removed
		assert!(!Multisigs::<T>::contains_key(&multisig_account), "Multisig not destroyed");

		// Check that owner mappings were removed
		for owner in owners.iter() {
			assert!(
				!MultisigOwners::<T>::contains_key(&multisig_account, owner),
				"Owner mapping not removed"
			);
		}

		// Check that transactions were removed
		for i in 0..t {
			assert!(
				!Transactions::<T>::contains_key(&multisig_account, i),
				"Transaction not removed"
			);
		}
	}

	impl_benchmark_test_suite!(Multisig, crate::mock::new_test_ext(), crate::mock::Test,);
}
