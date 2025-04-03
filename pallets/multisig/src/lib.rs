#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html
// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html
// https://paritytech.github.io/polkadot-sdk/master/frame_support/attr.pallet.html#dev-mode-palletdev_mode
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use frame_support::{
		dispatch::{GetDispatchInfo, RawOrigin},
		pallet_prelude::*,
		traits::{fungible, fungible::MutateHold, Contains, VariantCount},
	};
	use frame_system::pallet_prelude::*;
	use sp_io::hashing::blake2_256;
	use sp_runtime::{
		traits::{Dispatchable, TrailingZeroInput},
		Saturating,
	};
	use sp_std::{collections::btree_set::BTreeSet, prelude::*};
	// use sp_arithmetic::traits::{Arithmetic, Saturating};

	pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[derive(Copy, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub enum HoldReason {
		Multisig,
	}

	// Implemented VariantCount manually as the VariantCount macro is in variant_count crate which
	// is not available
	impl VariantCount for HoldReason {
		const VARIANT_COUNT: u32 = 1; // Because we have only one variant (Multisig)
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/reference_docs/frame_runtime_types/index.html
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type to access the Balances Pallet.
		type NativeBalance: fungible::Inspect<Self::AccountId>
			+ fungible::Mutate<Self::AccountId>
			+ fungible::hold::Inspect<Self::AccountId, Reason = HoldReason>
			+ fungible::hold::Mutate<Self::AccountId>
			+ fungible::freeze::Inspect<Self::AccountId>
			+ fungible::freeze::Mutate<Self::AccountId>;

		/// A type representing all calls available in your runtime.
		/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/reference_docs/frame_runtime_types/index.html
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo;

		/// The maximum number of owners per multisig.
		#[pallet::constant]
		type MaxOwners: Get<u32>; // TODO: review

		/// Base deposit for creating a multisig
		#[pallet::constant]
		type MultisigDepositBase: Get<BalanceOf<Self>>;

		/// Deposit factor per owner
		#[pallet::constant]
		type MultisigDepositFactor: Get<BalanceOf<Self>>;

		/// Filter for allowed calls in multisig transactions
		type CallFilter: Contains<<Self as frame_system::Config>::RuntimeCall>;
	}

	/// A multisig wallet and its configuration
	#[derive(Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Multisig<T: Config> {
		/// The multisig account ID
		pub account_id: T::AccountId,
		/// The list of owners
		pub owners: BoundedVec<T::AccountId, T::MaxOwners>,
		/// Number of required confirmations
		pub required: u32,
		/// Transaction count
		pub transaction_count: u32,
	}

	// Implement Clone manually to ensure proper trait bounds
	impl<T: Config> Clone for Multisig<T> {
		fn clone(&self) -> Self {
			Self {
				account_id: self.account_id.clone(),
				owners: self.owners.clone(),
				required: self.required,
				transaction_count: self.transaction_count,
			}
		}
	}

	/// A transaction proposal in a multisig
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Transaction<T: Config> {
		/// The encoded call to be dispatched
		pub call: <T as frame_system::Config>::RuntimeCall,
		/// True if the transaction has been executed
		pub executed: bool,
	}

	/// The pallet's storage items.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#storage
	/// https://paritytech.github.io/polkadot-sdk/master/frame_support/pallet_macros/attr.storage.html

	#[pallet::storage]
	pub type Multisigs<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Multisig<T>>;

	#[pallet::storage]
	pub type LastMultisigSeed<T> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	pub type MultisigOwners<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId, // Multisig account ID
		Twox64Concat,
		T::AccountId, // Owner account ID
		bool,
	>;

	#[pallet::storage]
	pub type Transactions<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId, // Multisig account ID
		Twox64Concat,
		u32, // Transaction ID
		Transaction<T>,
	>;

	#[pallet::storage]
	pub type Confirmations<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId, // Multisig account ID
		Twox64Concat,
		(u32, T::AccountId), // (Transaction ID, Owner)
		bool,
	>;

	/// Pallets use events to inform users when important changes are made.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new multisig was created
		MultisigCreated { multisig: T::AccountId, creator: T::AccountId, threshold: u32 },
		/// A transaction was submitted to a multisig
		TransactionSubmitted {
			multisig: T::AccountId,
			transaction_id: u32,
			submitter: T::AccountId,
		},
		/// A transaction was executed
		TransactionExecuted { multisig: T::AccountId, transaction_id: u32, result: DispatchResult },
		/// A transaction was signed by an owner
		TransactionSigned { multisig: T::AccountId, transaction_id: u32, signer: T::AccountId },
		/// A transaction was unsigned by an owner
		TransactionUnsigned { multisig: T::AccountId, transaction_id: u32, signer: T::AccountId }, /* TODO: rename to TransactionRevoked in rafac */
		/// A new owner was added to a multisig
		OwnerAdded { multisig: T::AccountId, new_owner: T::AccountId },
		/// Requirement of a multisig was changed
		RequirementChanged { multisig: T::AccountId, new_threshold: u32 },
		/// An owner was removed from a multisig
		OwnerRemoved { multisig: T::AccountId, old_owner: T::AccountId },
		/// A multisig was destroyed
		MultisigDestroyed { multisig: T::AccountId, approving_account: T::AccountId },
	}

	/// Errors inform users that something went wrong.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error
	#[pallet::error]
	pub enum Error<T> {
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		/// Threshold must be greater than 0 and less than or equal to the number of owners
		InvalidThreshold,
		/// Too many owners in the multisig
		TooManyOwners,
		/// Owner already exists in the multisig or duplicate owners provided
		AlreadyOwner,
		/// Failed to reserve deposit
		FailedToReserveDeposit,
		/// Multisig not found
		MultisigNotFound,
		/// Not an owner of the multisig
		NotOwner,
		/// Transaction already executed
		AlreadyExecuted,
		/// Transaction not found
		TransactionNotFound,
		/// Transaction already confirmed
		AlreadyConfirmed,
		/// Transaction not confirmed
		NotConfirmed,
		/// Owner not found
		OwnerNotFound,
		/// Not allowed to dissolve
		NotAllowedToDissolve,
		/// Call is not allowed by the multisig filter
		CallNotAllowed,
	}

	/// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	/// These functions materialize as "extrinsics", which are often compared to transactions.
	/// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#dispatchables
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new multisig wallet
		#[pallet::call_index(0)]
		pub fn create_multisig(
			origin: OriginFor<T>,
			owners: Vec<T::AccountId>,
			threshold: u32,
		) -> DispatchResult {
			// ensure signed
			let creator = ensure_signed(origin)?;

			// Ensure valid parameters
			ensure!(!owners.is_empty(), Error::<T>::InvalidThreshold);
			ensure!(owners.len() <= T::MaxOwners::get() as usize, Error::<T>::TooManyOwners);
			ensure!(
				threshold > 0 && threshold <= owners.len() as u32,
				Error::<T>::InvalidThreshold
			);

			// Ensure owner uniqueness
			let mut unique_owners = BTreeSet::new();
			for owner in owners.iter() {
				ensure!(unique_owners.insert(owner.clone()), Error::<T>::AlreadyOwner);
			}

			// Generate a unique address for the multisig
			let seed = LastMultisigSeed::<T>::get();
			let multisig_account = Self::multi_account_id(seed);
			LastMultisigSeed::<T>::put(seed + 1);

			// Calculate and hold the deposit
			let deposit = Self::calculate_deposit(owners.len() as u32)?;
			T::NativeBalance::hold(&HoldReason::Multisig, &creator, deposit)
				.map_err(|_| Error::<T>::FailedToReserveDeposit)?;

			// Create the multisig
			let owners_bounded = BoundedVec::<T::AccountId, T::MaxOwners>::try_from(owners)
				.map_err(|_| Error::<T>::TooManyOwners)?;

			let multisig = Multisig {
				account_id: multisig_account.clone(),
				owners: owners_bounded.clone(),
				required: threshold,
				transaction_count: 0,
			};

			// Store the multisig
			<Multisigs<T>>::insert(&multisig_account, multisig);

			// Store owner mappings
			for owner in owners_bounded.iter() {
				<MultisigOwners<T>>::insert(&multisig_account, owner, true);
			}

			// Emit creation event
			Self::deposit_event(Event::MultisigCreated {
				multisig: multisig_account,
				creator,
				threshold,
			});

			Ok(())
		}

		/// Submit a transaction to a multisig
		#[pallet::call_index(1)]
		pub fn submit_transaction(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			call: Box<<T as frame_system::Config>::RuntimeCall>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Ensure the multisig exists
			let mut multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Ensure the sender is an owner
			ensure!(<MultisigOwners<T>>::contains_key(&multisig, &who), Error::<T>::NotOwner);

			// Check if the call is allowed by the filter
			ensure!(T::CallFilter::contains(&*call), Error::<T>::CallNotAllowed);

			// Create transaction ID and store transaction
			let transaction_id = multisig_data.transaction_count; // TODO: change to u128
														 // Increment transaction count
			multisig_data.transaction_count = multisig_data
				.transaction_count
				.checked_add(1)
				.ok_or("Transaction count overflow")?;

			let transaction = Transaction { call: *call, executed: false };

			// Store transaction
			<Transactions<T>>::insert(&multisig, transaction_id, transaction);
			// Store multisig with updated transaction count
			<Multisigs<T>>::insert(&multisig, multisig_data.clone());

			// Automatically confirm the transaction for the submitter
			<Confirmations<T>>::insert(&multisig, (transaction_id, who.clone()), true);

			// Emit events
			Self::deposit_event(Event::TransactionSubmitted {
				multisig: multisig.clone(),
				transaction_id,
				submitter: who.clone(),
			});
			// TransactionSigned event is emitted
			Self::deposit_event(Event::TransactionSigned {
				multisig: multisig.clone(),
				transaction_id,
				signer: who.clone(),
			});

			// Check if we have enough confirmations to execute
			let confirmation_count = Self::count_confirmations(&multisig, transaction_id);

			if confirmation_count >= multisig_data.required {
				Self::do_execute_transaction(&multisig, transaction_id)?;
			}

			Ok(())
		}

		/// Confirm a multisig transaction
		#[pallet::call_index(2)]
		pub fn confirm_transaction(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			transaction_id: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Ensure the multisig exists
			let multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Ensure the sender is an owner
			ensure!(<MultisigOwners<T>>::contains_key(&multisig, &who), Error::<T>::NotOwner);

			// Ensure the transaction exists and is not executed
			let transaction = <Transactions<T>>::get(&multisig, transaction_id)
				.ok_or(Error::<T>::TransactionNotFound)?;
			ensure!(!transaction.executed, Error::<T>::AlreadyExecuted);

			// Ensure the signer has not confirmed yet
			ensure!(
				!<Confirmations<T>>::contains_key(&multisig, (transaction_id, who.clone())),
				Error::<T>::AlreadyConfirmed
			);

			// Store confirmation
			<Confirmations<T>>::insert(&multisig, (transaction_id, who.clone()), true);

			// Emit confirmation event
			Self::deposit_event(Event::TransactionSigned {
				multisig: multisig.clone(),
				transaction_id,
				signer: who.clone(),
			});

			// Check if we have enough confirmations to execute
			let confirmation_count = Self::count_confirmations(&multisig, transaction_id);

			if confirmation_count >= multisig_data.required {
				Self::do_execute_transaction(&multisig, transaction_id)?;
			}

			Ok(())
		}

		/// Revoke a previous confirmation
		#[pallet::call_index(3)]
		pub fn revoke_confirmation(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			transaction_id: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Ensure the multisig exists
			let _multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Ensure the sender is an owner
			ensure!(<MultisigOwners<T>>::contains_key(&multisig, &who), Error::<T>::NotOwner);

			// Ensure the transaction exists and is not executed
			let transaction = <Transactions<T>>::get(&multisig, transaction_id)
				.ok_or(Error::<T>::TransactionNotFound)?;
			ensure!(!transaction.executed, Error::<T>::AlreadyExecuted);

			// Ensure the signer has confirmed previously
			ensure!(
				<Confirmations<T>>::contains_key(&multisig, (transaction_id, who.clone())),
				Error::<T>::NotConfirmed
			);

			// Remove confirmation
			<Confirmations<T>>::remove(&multisig, (transaction_id, who.clone()));

			// Emit revocation event
			Self::deposit_event(Event::TransactionUnsigned {
				multisig,
				transaction_id,
				signer: who,
			});

			Ok(())
		}

		/// Add a new owner to a multisig
		#[pallet::call_index(4)]
		pub fn add_owner(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			new_owner: T::AccountId,
		) -> DispatchResult {
			// Only allow calls from the multisig itself
			Self::ensure_multisig(&multisig, origin)?;

			// Ensure the multisig exists
			let mut multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Ensure the new owner is not already an owner
			ensure!(
				!<MultisigOwners<T>>::contains_key(&multisig, &new_owner),
				Error::<T>::AlreadyOwner
			);

			// Ensure we don't exceed max owners
			ensure!(
				multisig_data.owners.len() < T::MaxOwners::get() as usize,
				Error::<T>::TooManyOwners
			);

			// Add the new owner
			multisig_data
				.owners
				.try_push(new_owner.clone())
				.map_err(|_| Error::<T>::TooManyOwners)?;

			// Update storage
			<Multisigs<T>>::insert(&multisig, multisig_data);
			<MultisigOwners<T>>::insert(&multisig, &new_owner, true);

			// Emit event
			Self::deposit_event(Event::OwnerAdded { multisig, new_owner });

			Ok(())
		}

		/// Remove an owner from the multisig
		#[pallet::call_index(5)]
		pub fn remove_owner(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			old_owner: T::AccountId,
		) -> DispatchResult {
			// Only allow calls from the multisig itself
			Self::ensure_multisig(&multisig, origin)?;

			// Ensure the multisig exists
			let mut multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Ensure the old owner is actually an owner
			ensure!(
				<MultisigOwners<T>>::contains_key(&multisig, &old_owner),
				Error::<T>::OwnerNotFound
			);

			// Remove the owner
			multisig_data.owners.retain(|o| o != &old_owner);

			// Adjust required confirmations if needed
			if multisig_data.required > multisig_data.owners.len() as u32 {
				multisig_data.required = multisig_data.owners.len() as u32;
				Self::deposit_event(Event::RequirementChanged {
					multisig: multisig.clone(),
					new_threshold: multisig_data.required,
				});
			}

			// Update storage
			<Multisigs<T>>::insert(&multisig, multisig_data);
			<MultisigOwners<T>>::remove(&multisig, &old_owner);

			// Emit event
			Self::deposit_event(Event::OwnerRemoved { multisig, old_owner });

			Ok(())
		}

		/// Replace an owner in the multisig
		#[pallet::call_index(6)]
		pub fn replace_owner(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			old_owner: T::AccountId,
			new_owner: T::AccountId,
		) -> DispatchResult {
			// Only allow calls from the multisig itself
			Self::ensure_multisig(&multisig, origin)?;

			// Ensure the multisig exists
			let mut multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Ensure the old owner exists and new owner doesn't exist
			ensure!(
				<MultisigOwners<T>>::contains_key(&multisig, &old_owner),
				Error::<T>::OwnerNotFound
			);
			ensure!(
				!<MultisigOwners<T>>::contains_key(&multisig, &new_owner),
				Error::<T>::AlreadyOwner
			);

			// Replace the owner
			for i in 0..multisig_data.owners.len() {
				if multisig_data.owners[i] == old_owner {
					multisig_data.owners[i] = new_owner.clone();
					break;
				}
			}

			// Update storage
			<Multisigs<T>>::insert(&multisig, multisig_data);
			<MultisigOwners<T>>::remove(&multisig, &old_owner);
			<MultisigOwners<T>>::insert(&multisig, &new_owner, true);

			// Emit events
			Self::deposit_event(Event::OwnerRemoved { multisig: multisig.clone(), old_owner });
			Self::deposit_event(Event::OwnerAdded { multisig, new_owner });

			Ok(())
		}

		/// Change the required confirmations threshold
		#[pallet::call_index(7)]
		pub fn change_requirement(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			new_threshold: u32,
		) -> DispatchResult {
			// Only allow calls from the multisig itself
			Self::ensure_multisig(&multisig, origin)?;

			// Ensure the multisig exists
			let mut multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Validate the new threshold
			ensure!(
				new_threshold > 0 && new_threshold <= multisig_data.owners.len() as u32,
				Error::<T>::InvalidThreshold
			);

			// Update the threshold
			multisig_data.required = new_threshold;
			<Multisigs<T>>::insert(&multisig, multisig_data);

			// Emit event
			Self::deposit_event(Event::RequirementChanged { multisig, new_threshold });

			Ok(())
		}

		/// Permanently destroy a multisig wallet
		#[pallet::call_index(8)]
		pub fn destroy_multisig(origin: OriginFor<T>, multisig: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Ensure the multisig exists
			let multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Only allow owners to dissolve
			ensure!(
				<MultisigOwners<T>>::contains_key(&multisig, &who),
				Error::<T>::NotAllowedToDissolve
			);

			// Remove all storage items related to this multisig
			<Multisigs<T>>::remove(&multisig);

			// Clean up owner mappings
			for owner in multisig_data.owners.iter() {
				<MultisigOwners<T>>::remove(&multisig, owner);
			}

			// Clean up all transactions and confirmations
			for transaction_id in 0..multisig_data.transaction_count {
				<Transactions<T>>::remove(&multisig, transaction_id);

				// Clean up confirmations for this transaction
				for owner in multisig_data.owners.iter() {
					<Confirmations<T>>::remove(&multisig, (transaction_id, owner.clone()));
				}
			}

			// Emit event
			Self::deposit_event(Event::MultisigDestroyed { multisig, approving_account: who });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Derive a unique account id using a seed value.
		pub fn multi_account_id(seed: u32) -> T::AccountId {
			let entropy = (b"pba/multisig", seed).using_encoded(blake2_256);
			Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
				.expect("infinite length input; no invalid inputs for type; qed")
		}

		/// Calculate the deposit required for a multisig wallet
		pub fn calculate_deposit(num_owners: u32) -> Result<BalanceOf<T>, Error<T>> {
			let base = T::MultisigDepositBase::get();
			let factor = T::MultisigDepositFactor::get();
			let factor_mul = factor.saturating_mul(num_owners.into());
			Ok(base.saturating_add(factor_mul))
		}

		/// Count confirmations for a transaction
		pub fn count_confirmations(multisig: &T::AccountId, transaction_id: u32) -> u32 {
			let mut count = 0;

			if let Some(multisig_data) = <Multisigs<T>>::get(multisig) {
				for owner in multisig_data.owners.iter() {
					if <Confirmations<T>>::contains_key(multisig, (transaction_id, owner.clone())) {
						count += 1;
					}
				}
			}

			count
		}

		//// Helper to execute a transaction
		fn do_execute_transaction(multisig: &T::AccountId, transaction_id: u32) -> DispatchResult {
			// Get the transaction
			let mut transaction = <Transactions<T>>::get(multisig, transaction_id)
				.ok_or(Error::<T>::TransactionNotFound)?;
			ensure!(!transaction.executed, Error::<T>::AlreadyExecuted);

			// Mark as executed before actual execution to avoid reentrancy attacks
			transaction.executed = true;
			<Transactions<T>>::insert(multisig, transaction_id, transaction.clone());

			// Execute the call i.e redispatch it
			let result = transaction.call.dispatch(RawOrigin::Signed(multisig.clone()).into()); // redispatching the call on behalf of the multisig

			// Emit event with the result
			let dispatch_result = result.map(|_| ()).map_err(|e| e.error);
			Self::deposit_event(Event::TransactionExecuted {
				multisig: multisig.clone(),
				transaction_id,
				result: dispatch_result,
			});

			// If execution failed, don't propagate the error so that the transaction
			// remains marked as executed - this matches Gnosis behavior
			Ok(())
		}

		/// Check if the origin is a multisig account
		fn ensure_multisig(
			multisig: &T::AccountId,
			origin: OriginFor<T>,
		) -> Result<(), DispatchError> {
			let who = ensure_signed(origin)?;
			ensure!(who == *multisig, Error::<T>::NotOwner);
			Ok(())
		}
	}
}
