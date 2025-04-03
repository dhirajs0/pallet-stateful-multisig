use crate as pallet_multisig;
use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};

#[test]
fn call_filter_blocks_privileged_operations() {
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

		// Submit another privileged operation (set_code)
		let call =
			Box::new(RuntimeCall::System(frame_system::Call::set_code { code: vec![1, 2, 3] }));

		// Should also fail as privileged operations are blocked
		assert_noop!(
			pallet_multisig::Pallet::<Test>::submit_transaction(
				RuntimeOrigin::signed(1),
				multisig,
				call
			),
			Error::<Test>::CallNotAllowed
		);
	});
}
