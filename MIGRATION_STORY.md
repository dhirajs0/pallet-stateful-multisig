The current **stateless multisig pallet** (as implemented in Substrate and referenced in your snippet) provides basic multisignature transaction approval and execution using deterministic addresses and minimal on-chain state. However, this design comes with limitations that a **stateful multisig** pallet could resolve. Let's break it down.

---

## ✅ Features of the Current (Stateless) Multisig Pallet

1. **Composite Multisig Address**

   - Deterministically derives a multisig account ID using signatories and threshold.
   - This address is where the call is executed from once enough approvals are gathered.

2. **Flexible Thresholds**

   - Allows a configurable number of signers and threshold for approval.

3. **Call Approval Mechanism**

   - `as_multi`, `approve_as_multi`, and `cancel_as_multi` allow for flexible control over the approval and execution of a call.

4. **Call Hash Support**

   - Partial approvals can be stored using just the call hash (less storage, privacy until execution).

5. **Call Execution on Final Approval**

   - Executes the call once the threshold of approvals is met.

6. **Deposit-based Anti-spam**

   - Reserves a deposit from the initiator to prevent spam. Returned after execution or cancellation.


---

## ❌ Limitations Compared to a Stateful Multisig

The current pallet is **not stateful at the multisig account level**. Here's a detailed list of limitations and shortcomings that a **stateful multisig design would address**:

---

### 1. **No On-chain Record of Membership or Threshold**

- There's no persistent on-chain state representing a _multisig entity_.

🡒 **Stateful multisig** would allow:

- Creating a persistent multisig group once.
- Reusing it across multiple calls.
- Updating group members or thresholds via consensus from the multisig group.

---

### 2. **No Support for Membership Management**

- Can't add or remove signatories, change threshold, or rotate keys.
- You'd need to re-create the multisig group from scratch for every change.

🡒 **Stateful multisig** enables dynamic governance:

- Add/remove signers, update threshold with group consensus.

---

### 3. **Limited Auditability & Usability**

- No list of all pending multisig actions per group.
- Difficult to query all multisig actions from a given multisig account.

🡒 **Stateful multisig** would store a list of:

- Pending proposals
- Historical actions
- Voting records

---

### 4. **No Proposal Metadata**

- Only the call hash or full call is stored.
- No title, description, or contextual info for proposals.

🡒 **Stateful multisig** could include metadata to describe proposals:

- e.g., "Transfer funds to treasury", with comments, tags.

---

### 5. **Repetition of Parameters**

- Must include all signatories and threshold in every call.

🡒 **Stateful multisig** needs this info once during group creation.

---

### 6. **Gas/Storage Inefficiency**

- Every operation stores redundant signer data.
- Separate storage entries for every multisig operation even within the same group.

🡒 **Stateful multisig** allows batching proposals and storing config once.

---
---

## 🔁 Migration Story: Stateless ➜ Stateful

To transition from the current stateless multisig pallet to a stateful version:

1. **Introduce Group Registration**

   - A new multisig group can be created with `create_multisig(threshold, members)`.

2. **Proposal Lifecycle**

   - `propose(group_id, call)`: Create a proposal
   - `approve(group_id, proposal_id)`
   - `execute(group_id, proposal_id)`

3. **Optional Migration Hook**

   - Add tooling to "import" existing stateless proposals or help users migrate their groups.

4. **State Reuse**
   - Stateless multisig addresses could still be supported for compatibility or read-only views.

---

## 🧩 TL;DR

| Feature                | Stateless Multisig | Stateful Multisig (Proposed) |
| ---------------------- | ------------------ | ---------------------------- |
| Persistent Group State | ❌                 | ✅                           |
| Membership Management  | ❌                 | ✅                           |
| Proposal History       | ❌                 | ✅                           |
| Metadata Support       | ❌                 | ✅                           |
| Parameter Reusability  | ❌                 | ✅                           |


---
