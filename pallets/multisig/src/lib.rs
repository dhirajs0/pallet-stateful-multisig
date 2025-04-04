#![cfg_attr(not(feature = "std"), no_std)]

//! # Stateful Multisig Pallet
//!
//! A pallet that provides functionality for creating and managing stateful multisignature wallets.
//!
//! ## Overview
//!
//! This pallet allows users to:
//! - Create multisig wallets with custom owners and threshold requirements
//! - Submit, confirm, and execute transactions through the multisig
//! - Manage ownership by adding, removing, or replacing owners
//! - Change confirmation requirements
//! - Destroy multisigs when no longer needed
//!
//! The design is inspired by the Gnosis Multisig contract with adaptations for the
//! Substrate ecosystem.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! * `create_multisig` - Create a new multisig wallet with specified owners and threshold
//! * `submit_transaction` - Submit a new transaction to a multisig for approval
//! * `confirm_transaction` - Confirm a pending transaction
//! * `revoke_confirmation` - Revoke a previously given confirmation
//! * `add_owner` - Add a new owner to a multisig (must be called by the multisig itself)
//! * `remove_owner` - Remove an owner from a multisig (must be called by the multisig itself)
//! * `replace_owner` - Replace an existing owner with a new one (must be called by the multisig
//!   itself)
//! * `change_requirement` - Change the confirmation threshold (must be called by the multisig
//!   itself)
//! * `destroy_multisig` - Permanently destroy a multisig wallet and release the deposit

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
		traits::{fungible, fungible::MutateHold, tokens::Precision, Contains, VariantCount},
	};
	use frame_system::pallet_prelude::*;
	use sp_io::hashing::blake2_256;
	use sp_runtime::{
		traits::{Dispatchable, TrailingZeroInput},
		Saturating,
	};
	use sp_std::{collections::btree_set::BTreeSet, prelude::*};

	pub type BalanceOf<T> = <<T as Config>::NativeBalance as fungible::Inspect<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	/// Reason for holding funds in a multisig wallet.
	#[derive(Copy, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub enum HoldReason {
		/// Funds are held for a multisig wallet deposit.
		Multisig,
	}

	// Implement VariantCount manually since the VariantCount macro isn't available
	impl VariantCount for HoldReason {
		const VARIANT_COUNT: u32 = 1; // Only one variant (Multisig)
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

		/// The maximum number of owners a multisig can have.
		#[pallet::constant]
		type MaxOwners: Get<u32>;

		/// The base deposit required to create a multisig.
		#[pallet::constant]
		type MultisigDepositBase: Get<BalanceOf<Self>>;

		/// The additional deposit per owner of a multisig.
		#[pallet::constant]
		type MultisigDepositFactor: Get<BalanceOf<Self>>;

		/// A filter that determines which calls can be made via multisig transactions.
		type CallFilter: Contains<<Self as frame_system::Config>::RuntimeCall>;
	}

	/// A multisig wallet and its configuration.
	///
	/// Holds all the data needed to identify and operate a multisig wallet.
	#[derive(Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Multisig<T: Config> {
		/// The account ID that represents this multisig
		pub account_id: T::AccountId,
		/// The list of owners who can approve transactions
		pub owners: BoundedVec<T::AccountId, T::MaxOwners>,
		/// Number of confirmations required to execute a transaction
		pub required: u32,
		/// Counter for transaction IDs
		pub transaction_count: u32,
		/// The account that created this multisig and paid the deposit
		pub creator: T::AccountId,
		/// Amount of funds held as deposit
		pub deposit: BalanceOf<T>,
	}

	// Implemented Clone manually to ensure proper trait bounds
	impl<T: Config> Clone for Multisig<T> {
		fn clone(&self) -> Self {
			Self {
				account_id: self.account_id.clone(),
				owners: self.owners.clone(),
				required: self.required,
				transaction_count: self.transaction_count,
				creator: self.creator.clone(),
				deposit: self.deposit,
			}
		}
	}

	/// A transaction proposal within a multisig wallet.
	#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Transaction<T: Config> {
		/// The call to be dispatched when the transaction is executed
		pub call: <T as frame_system::Config>::RuntimeCall,
		/// Whether the transaction has been executed
		pub executed: bool,
	}

	/// Stores the multisig wallet data, indexed by the multisig account ID.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#storage
	/// https://paritytech.github.io/polkadot-sdk/master/frame_support/pallet_macros/attr.storage.html

	#[pallet::storage]
	pub type Multisigs<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Multisig<T>>;

	/// Counter used to generate unique multisig account IDs.
	#[pallet::storage]
	pub type LastMultisigSeed<T> = StorageValue<_, u32, ValueQuery>;

	/// Maps multisig account IDs to their owners.
	/// This allows efficient checking of ownership status.
	#[pallet::storage]
	pub type MultisigOwners<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId, // Multisig account ID
		Twox64Concat,
		T::AccountId, // Owner account ID
		bool,
	>;

	/// Stores transaction data for a multisig.
	#[pallet::storage]
	pub type Transactions<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId, // Multisig account ID
		Twox64Concat,
		u32, // Transaction ID
		Transaction<T>,
	>;

	/// Tracks which owners have confirmed which transactions.
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
		/// A new multisig was created. \[multisig, creator, threshold\]
		MultisigCreated { multisig: T::AccountId, creator: T::AccountId, threshold: u32 },
		/// A transaction was submitted to a multisig. \[multisig, transaction_id, submitter\]
		TransactionSubmitted {
			multisig: T::AccountId,
			transaction_id: u32,
			submitter: T::AccountId,
		},
		/// A transaction was executed. \[multisig, transaction_id, result\]
		TransactionExecuted { multisig: T::AccountId, transaction_id: u32, result: DispatchResult },
		/// A transaction was signed by an owner. \[multisig, transaction_id, signer\]
		TransactionSigned { multisig: T::AccountId, transaction_id: u32, signer: T::AccountId },
		/// A transaction signature was revoked by an owner. \[multisig, transaction_id, signer\]
		TransactionRevoked { multisig: T::AccountId, transaction_id: u32, signer: T::AccountId },
		/// A new owner was added to a multisig. \[multisig, new_owner\]
		OwnerAdded { multisig: T::AccountId, new_owner: T::AccountId },
		/// Requirement of a multisig was changed. \[multisig, new_threshold\]
		RequirementChanged { multisig: T::AccountId, new_threshold: u32 },
		/// An owner was removed from a multisig. \[multisig, old_owner\]
		OwnerRemoved { multisig: T::AccountId, old_owner: T::AccountId },
		/// A multisig was destroyed. \[multisig, approving_account\]
		MultisigDestroyed { multisig: T::AccountId, approving_account: T::AccountId },
	}

	/// Errors inform users that something went wrong.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#event-and-error
	#[pallet::error]
	pub enum Error<T> {
		/// An operation would lead to a storage overflow.
		StorageOverflow,
		/// The provided threshold is invalid (must be > 0 and <= owner count).
		InvalidThreshold,
		/// The multisig would have too many owners.
		TooManyOwners,
		/// The account is already an owner or duplicate owners were provided.
		AlreadyOwner,
		/// Could not reserve the deposit for creating a multisig.
		FailedToReserveDeposit,
		/// The specified multisig doesn't exist.
		MultisigNotFound,
		/// The caller is not an owner of the multisig.
		NotOwner,
		/// The transaction has already been executed.
		AlreadyExecuted,
		/// The specified transaction doesn't exist.
		TransactionNotFound,
		/// The transaction is already confirmed by this owner.
		AlreadyConfirmed,
		/// The transaction is not confirmed by this owner.
		NotConfirmed,
		/// The specified owner doesn't exist in this multisig.
		OwnerNotFound,
		/// The caller is not allowed to dissolve this multisig.
		NotAllowedToDissolve,
		/// The call is not permitted by the multisig call filter.
		CallNotAllowed,
		/// Could not release the deposit when destroying the multisig.
		FailedToReleaseDeposit,
	}

	/// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	/// These functions materialize as "extrinsics", which are often compared to transactions.
	/// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	/// https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#dispatchables
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a new multisig wallet with specified owners and threshold.
		///
		/// * `owners` - The list of initial owners of the multisig.
		/// * `threshold` - The number of confirmations required for transaction execution.
		///
		/// # <weight>
		/// - O(O) where O is the number of owners
		/// - Limited storage reads/writes
		/// - One balance hold operation
		/// - Emits `MultisigCreated` event
		/// # </weight>
		#[pallet::call_index(0)]
		pub fn create_multisig(
			origin: OriginFor<T>,
			owners: Vec<T::AccountId>,
			threshold: u32,
		) -> DispatchResult {
			let creator = ensure_signed(origin)?;

			// Validate parameters
			ensure!(!owners.is_empty(), Error::<T>::InvalidThreshold);
			ensure!(owners.len() <= T::MaxOwners::get() as usize, Error::<T>::TooManyOwners);
			ensure!(
				threshold > 0 && threshold <= owners.len() as u32,
				Error::<T>::InvalidThreshold
			);

			// Check for duplicate owners
			let mut unique_owners = BTreeSet::new();
			for owner in owners.iter() {
				ensure!(unique_owners.insert(owner.clone()), Error::<T>::AlreadyOwner);
			}

			// Generate unique multisig account
			let seed = LastMultisigSeed::<T>::get();
			let multisig_account = Self::multi_account_id(seed);
			LastMultisigSeed::<T>::put(seed.saturating_add(1));

			// Calculate and hold deposit
			let deposit = Self::calculate_deposit(owners.len() as u32)?;
			T::NativeBalance::hold(&HoldReason::Multisig, &creator, deposit)
				.map_err(|_| Error::<T>::FailedToReserveDeposit)?;

			// Create multisig struct
			let owners_bounded = BoundedVec::<T::AccountId, T::MaxOwners>::try_from(owners)
				.map_err(|_| Error::<T>::TooManyOwners)?;

			let multisig = Multisig {
				account_id: multisig_account.clone(),
				owners: owners_bounded.clone(),
				required: threshold,
				transaction_count: 0,
				creator: creator.clone(),
				deposit,
			};

			// Store multisig data
			<Multisigs<T>>::insert(&multisig_account, multisig);

			// Store owner mappings
			for owner in owners_bounded.iter() {
				<MultisigOwners<T>>::insert(&multisig_account, owner, true);
			}

			// Emit event
			Self::deposit_event(Event::MultisigCreated {
				multisig: multisig_account,
				creator,
				threshold,
			});

			Ok(())
		}

		/// Submit a transaction to a multisig for approval.
		///
		/// The submitter must be an owner of the multisig. The transaction is automatically
		/// confirmed by the submitter. If the submission pushes confirmations over the threshold,
		/// the transaction will be executed.
		///
		/// * `multisig` - The account ID of the multisig wallet.
		/// * `call` - The call to be executed when the transaction is approved.
		///
		/// # <weight>
		/// - Complexity: O(1)
		/// - Storage reads/writes depend on confirmation count
		/// - May execute a transaction if threshold is met
		/// - Emits `TransactionSubmitted` and `TransactionSigned` events
		/// # </weight>
		#[pallet::call_index(1)]
		pub fn submit_transaction(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			call: Box<<T as frame_system::Config>::RuntimeCall>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Get multisig data
			let mut multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Ensure caller is an owner
			ensure!(<MultisigOwners<T>>::contains_key(&multisig, &who), Error::<T>::NotOwner);

			// Ensure call is allowed
			ensure!(T::CallFilter::contains(&*call), Error::<T>::CallNotAllowed);

			// Create transaction ID
			let transaction_id = multisig_data.transaction_count;

			// Safely increment transaction count
			multisig_data.transaction_count = multisig_data
				.transaction_count
				.checked_add(1)
				.ok_or(Error::<T>::StorageOverflow)?;

			// Create and store transaction
			let transaction = Transaction { call: *call, executed: false };

			// Store transaction
			<Transactions<T>>::insert(&multisig, transaction_id, transaction);

			// Update multisig with new transaction count
			<Multisigs<T>>::insert(&multisig, multisig_data.clone());

			// Auto-confirm for submitter
			<Confirmations<T>>::insert(&multisig, (transaction_id, who.clone()), true);

			// Emit events
			Self::deposit_event(Event::TransactionSubmitted {
				multisig: multisig.clone(),
				transaction_id,
				submitter: who.clone(),
			});

			Self::deposit_event(Event::TransactionSigned {
				multisig: multisig.clone(),
				transaction_id,
				signer: who,
			});

			// Check if we can execute the transaction
			let confirmation_count = Self::count_confirmations(&multisig, transaction_id);
			if confirmation_count >= multisig_data.required {
				Self::do_execute_transaction(&multisig, transaction_id)?;
			}

			Ok(())
		}

		/// Confirm a transaction that was previously submitted.
		///
		/// * `multisig` - The account ID of the multisig wallet.
		/// * `transaction_id` - The ID of the transaction to confirm.
		///
		/// # <weight>
		/// - Complexity: O(1)
		/// - Storage reads/writes depend on confirmation count
		/// - May execute a transaction if threshold is met
		/// - Emits `TransactionSigned` event
		/// # </weight>
		#[pallet::call_index(2)]
		pub fn confirm_transaction(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			transaction_id: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Verify multisig exists
			let multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Verify caller is an owner
			ensure!(<MultisigOwners<T>>::contains_key(&multisig, &who), Error::<T>::NotOwner);

			// Verify transaction exists and is not executed
			let transaction = <Transactions<T>>::get(&multisig, transaction_id)
				.ok_or(Error::<T>::TransactionNotFound)?;
			ensure!(!transaction.executed, Error::<T>::AlreadyExecuted);

			// Verify not already confirmed
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
				signer: who,
			});

			// Check if we have enough confirmations to execute
			let confirmation_count = Self::count_confirmations(&multisig, transaction_id);
			if confirmation_count >= multisig_data.required {
				Self::do_execute_transaction(&multisig, transaction_id)?;
			}

			Ok(())
		}

		/// Revoke a previously given confirmation.
		///
		/// * `multisig` - The account ID of the multisig wallet.
		/// * `transaction_id` - The ID of the transaction to revoke the confirmation from.
		///
		/// # <weight>
		/// - Complexity: O(1)
		/// - Limited storage reads/writes
		/// - Emits `TransactionRevoked` event
		/// # </weight>
		#[pallet::call_index(3)]
		pub fn revoke_confirmation(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			transaction_id: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Verify multisig exists
			let _multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Verify caller is an owner
			ensure!(<MultisigOwners<T>>::contains_key(&multisig, &who), Error::<T>::NotOwner);

			// Verify transaction exists and is not executed
			let transaction = <Transactions<T>>::get(&multisig, transaction_id)
				.ok_or(Error::<T>::TransactionNotFound)?;
			ensure!(!transaction.executed, Error::<T>::AlreadyExecuted);

			// Verify previously confirmed
			ensure!(
				<Confirmations<T>>::contains_key(&multisig, (transaction_id, who.clone())),
				Error::<T>::NotConfirmed
			);

			// Remove confirmation
			<Confirmations<T>>::remove(&multisig, (transaction_id, who.clone()));

			// Emit revocation event
			Self::deposit_event(Event::TransactionRevoked {
				multisig,
				transaction_id,
				signer: who,
			});

			Ok(())
		}

		/// Add a new owner to the multisig.
		///
		/// Must be called by the multisig account itself, meaning this must be
		/// part of an executed multisig transaction.
		///
		/// * `multisig` - The account ID of the multisig wallet (must match the caller).
		/// * `new_owner` - The account ID of the new owner to add.
		///
		/// # <weight>
		/// - O(O) where O is the number of owners
		/// - Limited storage reads/writes
		/// - Emits `OwnerAdded` event
		/// # </weight>
		#[pallet::call_index(4)]
		pub fn add_owner(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			new_owner: T::AccountId,
		) -> DispatchResult {
			// Only allow calls from the multisig itself
			Self::ensure_multisig(&multisig, origin)?;

			// Get multisig data
			let mut multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Verify new owner is not already an owner
			ensure!(
				!<MultisigOwners<T>>::contains_key(&multisig, &new_owner),
				Error::<T>::AlreadyOwner
			);

			// Verify we don't exceed max owners
			ensure!(
				multisig_data.owners.len() < T::MaxOwners::get() as usize,
				Error::<T>::TooManyOwners
			);

			// Add new owner
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

		/// Remove an owner from the multisig.
		///
		/// Must be called by the multisig account itself, meaning this must be
		/// part of an executed multisig transaction. If removing an owner causes
		/// the threshold to be greater than the remaining owners, the threshold
		/// will be automatically lowered.
		///
		/// * `multisig` - The account ID of the multisig wallet (must match the caller).
		/// * `old_owner` - The account ID of the owner to remove.
		///
		/// # <weight>
		/// - O(O) where O is the number of owners
		/// - Limited storage reads/writes
		/// - Emits `OwnerRemoved` event and possibly `RequirementChanged`
		/// # </weight>
		#[pallet::call_index(5)]
		pub fn remove_owner(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			old_owner: T::AccountId,
		) -> DispatchResult {
			// Only allow calls from the multisig itself
			Self::ensure_multisig(&multisig, origin)?;

			// Get multisig data
			let mut multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Verify old owner exists
			ensure!(
				<MultisigOwners<T>>::contains_key(&multisig, &old_owner),
				Error::<T>::OwnerNotFound
			);

			// Remove owner
			multisig_data.owners.retain(|o| o != &old_owner);

			// Adjust threshold if needed
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

		/// Replace an owner with a new owner in the multisig.
		///
		/// Must be called by the multisig account itself, meaning this must be
		/// part of an executed multisig transaction.
		///
		/// * `multisig` - The account ID of the multisig wallet (must match the caller).
		/// * `old_owner` - The account ID of the owner to replace.
		/// * `new_owner` - The account ID of the new owner.
		///
		/// # <weight>
		/// - O(O) where O is the number of owners
		/// - Limited storage reads/writes
		/// - Emits `OwnerRemoved` and `OwnerAdded` events
		/// # </weight>
		#[pallet::call_index(6)]
		pub fn replace_owner(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			old_owner: T::AccountId,
			new_owner: T::AccountId,
		) -> DispatchResult {
			// Only allow calls from the multisig itself
			Self::ensure_multisig(&multisig, origin)?;

			// Get multisig data
			let mut multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Verify owners
			ensure!(
				<MultisigOwners<T>>::contains_key(&multisig, &old_owner),
				Error::<T>::OwnerNotFound
			);
			ensure!(
				!<MultisigOwners<T>>::contains_key(&multisig, &new_owner),
				Error::<T>::AlreadyOwner
			);

			// Replace owner
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

		/// Change the confirmation threshold.
		///
		/// Must be called by the multisig account itself, meaning this must be
		/// part of an executed multisig transaction.
		///
		/// * `multisig` - The account ID of the multisig wallet (must match the caller).
		/// * `new_threshold` - The new threshold for confirmations.
		///
		/// # <weight>
		/// - Complexity: O(1)
		/// - Limited storage reads/writes
		/// - Emits `RequirementChanged` event
		/// # </weight>
		#[pallet::call_index(7)]
		pub fn change_requirement(
			origin: OriginFor<T>,
			multisig: T::AccountId,
			new_threshold: u32,
		) -> DispatchResult {
			// Only allow calls from the multisig itself
			Self::ensure_multisig(&multisig, origin)?;

			// Get multisig data
			let mut multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Validate threshold
			ensure!(
				new_threshold > 0 && new_threshold <= multisig_data.owners.len() as u32,
				Error::<T>::InvalidThreshold
			);

			// Update threshold
			multisig_data.required = new_threshold;
			<Multisigs<T>>::insert(&multisig, multisig_data);

			// Emit event
			Self::deposit_event(Event::RequirementChanged { multisig, new_threshold });

			Ok(())
		}

		/// Permanently destroy a multisig wallet.
		///
		/// Must be called by one of the multisig owners. Will release the deposit
		/// back to the creator account.
		///
		/// * `multisig` - The account ID of the multisig wallet to destroy.
		///
		/// # <weight>
		/// - Complexity: O(O + T) where O is the number of owners and T is the number of
		///   transactions
		/// - Storage cleanup operations
		/// - One balance release operation
		/// - Emits `MultisigDestroyed` event
		/// # </weight>
		#[pallet::call_index(8)]
		pub fn destroy_multisig(origin: OriginFor<T>, multisig: T::AccountId) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Get multisig data
			let multisig_data =
				<Multisigs<T>>::get(&multisig).ok_or(Error::<T>::MultisigNotFound)?;

			// Verify caller is an owner
			ensure!(
				<MultisigOwners<T>>::contains_key(&multisig, &who),
				Error::<T>::NotAllowedToDissolve
			);

			// Release the deposit
			T::NativeBalance::release(
				&HoldReason::Multisig,
				&multisig_data.creator,
				multisig_data.deposit,
				Precision::Exact,
			)
			.map_err(|_| Error::<T>::FailedToReleaseDeposit)?;

			// Clean up storage
			<Multisigs<T>>::remove(&multisig);

			// Remove owner mappings
			for owner in multisig_data.owners.iter() {
				<MultisigOwners<T>>::remove(&multisig, owner);
			}

			// Clean up transactions and confirmations
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
		/// Derives a unique account ID for a multisig wallet using the provided seed.
		///
		/// This uses the blake2_256 hashing algorithm to generate a deterministic
		/// account ID that can be recreated given the same seed.
		///
		/// # Arguments
		/// * `seed` - A unique seed value to generate the account ID.
		///
		/// # Returns
		/// The generated account ID.
		pub fn multi_account_id(seed: u32) -> T::AccountId {
			let entropy = (b"pba/multisig", seed).using_encoded(blake2_256);
			Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
				.expect("infinite length input; no invalid inputs for type; qed")
		}

		/// Calculates the required deposit for a multisig wallet based on the number of owners.
		///
		/// The deposit is calculated as: base_deposit + (owner_count * factor_deposit)
		///
		/// # Arguments
		/// * `num_owners` - The number of owners in the multisig.
		///
		/// # Returns
		/// The required deposit amount, or an error if calculation fails.
		pub fn calculate_deposit(num_owners: u32) -> Result<BalanceOf<T>, Error<T>> {
			let base = T::MultisigDepositBase::get();
			let factor = T::MultisigDepositFactor::get();
			let factor_mul = factor.saturating_mul(num_owners.into());
			Ok(base.saturating_add(factor_mul))
		}

		/// Counts the number of confirmations for a transaction.
		///
		/// # Arguments
		/// * `multisig` - The account ID of the multisig wallet.
		/// * `transaction_id` - The ID of the transaction to count confirmations for.
		///
		/// # Returns
		/// The number of confirmations for the transaction.
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

		/// Executes a transaction if it has enough confirmations.
		///
		/// This function marks the transaction as executed before dispatching
		/// to prevent reentrancy attacks. Even if execution fails, the transaction
		/// remains marked as executed.
		///
		/// # Arguments
		/// * `multisig` - The account ID of the multisig wallet.
		/// * `transaction_id` - The ID of the transaction to execute.
		///
		/// # Returns
		/// A dispatch result indicating success or failure.
		fn do_execute_transaction(multisig: &T::AccountId, transaction_id: u32) -> DispatchResult {
			// Get transaction
			let mut transaction = <Transactions<T>>::get(multisig, transaction_id)
				.ok_or(Error::<T>::TransactionNotFound)?;
			ensure!(!transaction.executed, Error::<T>::AlreadyExecuted);

			// Mark as executed before dispatching to prevent reentrancy
			transaction.executed = true;
			<Transactions<T>>::insert(multisig, transaction_id, transaction.clone());

			// Execute the call
			let result = transaction.call.dispatch(RawOrigin::Signed(multisig.clone()).into());

			// Emit event with result
			let dispatch_result = result.map(|_| ()).map_err(|e| e.error);
			Self::deposit_event(Event::TransactionExecuted {
				multisig: multisig.clone(),
				transaction_id,
				result: dispatch_result,
			});

			// Do not propagate execution errors to allow transaction to remain executed
			Ok(())
		}

		/// Verifies that the origin is a multisig account.
		///
		/// # Arguments
		/// * `multisig` - The expected multisig account ID.
		/// * `origin` - The origin to check.
		///
		/// # Returns
		/// Ok if the origin is the multisig account, Error otherwise.
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
