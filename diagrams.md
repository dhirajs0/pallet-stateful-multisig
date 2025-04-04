# Diagrams

<!-- add link to each diagram in this file -->

- [Multisig Data Model](#multisig-data-model)
- [Multisig Sequence Diagram](#multisig-sequence-diagram)
- [Create Multisig Flow](#create-multisig-flow)
- [Submit Transaction Flow](#submit-transaction-flow)
- [Confirm Transaction Flow](#confirm-transaction-flow)
- [Revoke Confirmation Flow](#revoke-confirmation-flow)
- [Add Owner Flow](#add-owner-flow)
- [Remove Owner Flow](#remove-owner-flow)
- [Replace Owner Flow](#replace-owner-flow)
- [Destroy Multisig Flow](#destroy-multisig-flow)
- [Change Requirement Flow](#change-requirement-flow)

## Multisig Data Model

```mermaid
classDiagram
    class Multisig {
        +AccountId account_id
        +BoundedVec~AccountId, MaxOwners~ owners
        +u32 required
        +u32 transaction_count
        +AccountId creator
        +Balance deposit
    }

    class Transaction {
        +RuntimeCall call
        +bool executed
    }

    class HoldReason {
        <<enum>>
        +Multisig
    }

    class StorageMaps {
        +Multisigs: Map~AccountId, Multisig~
        +MultisigOwners: DoubleMap~AccountId, AccountId, bool~
        +Transactions: DoubleMap~AccountId, u32, Transaction~
        +Confirmations: DoubleMap~AccountId, (u32, AccountId), bool~
        +LastMultisigSeed: Value~u32~
    }

    class Config {
        +type RuntimeEvent
        +type NativeBalance
        +type RuntimeCall
        +type MaxOwners: Get~u32~
        +type MultisigDepositBase: Get~Balance~
        +type MultisigDepositFactor: Get~Balance~
        +type CallFilter: Contains~RuntimeCall~
    }

    class Pallet {
        +create_multisig(owners, threshold)
        +submit_transaction(multisig, call)
        +confirm_transaction(multisig, transaction_id)
        +revoke_confirmation(multisig, transaction_id)
        +add_owner(multisig, new_owner)
        +remove_owner(multisig, old_owner)
        +replace_owner(multisig, old_owner, new_owner)
        +change_requirement(multisig, new_threshold)
        +destroy_multisig(multisig)
    }

    class HelperFunctions {
        -multi_account_id(seed)
        -calculate_deposit(num_owners)
        -count_confirmations(multisig, transaction_id)
        -do_execute_transaction(multisig, transaction_id)
        -ensure_multisig(multisig, origin)
    }

    Pallet --> Config: configured by
    Pallet --> StorageMaps: uses
    Pallet --> HelperFunctions: calls
    StorageMaps --> Multisig: stores
    StorageMaps --> Transaction: stores
    Multisig --> Transaction: "1" to "*"
```

## Multisig Sequence Diagram

```mermaid
sequenceDiagram
    participant Alice
    participant Bob
    participant Charlie
    participant MultisigPallet
    participant Runtime

    Alice->>MultisigPallet: create_multisig([Alice,Bob,Charlie], threshold=2)
    MultisigPallet-->>Alice: MultisigCreated event
    Note right of MultisigPallet: Multisig account created

    Note over Alice,MultisigPallet: Later...

    Alice->>MultisigPallet: submit_transaction(multisig, transfer_funds(to=Dave, amount=100))
    MultisigPallet-->>Alice: TransactionSubmitted & TransactionSigned events
    Note right of MultisigPallet: Submission counts as auto-confirmation

    Bob->>MultisigPallet: confirm_transaction(multisig, transaction_id)
    MultisigPallet-->>Bob: TransactionSigned event
    Note right of MultisigPallet: Threshold reached (2/3)

    MultisigPallet->>Runtime: dispatch(transfer_funds(to=Dave, amount=100))
    Runtime-->>MultisigPallet: execution_result
    MultisigPallet-->>Bob: TransactionExecuted event

    Note over Charlie,MultisigPallet: Management Example...

    Alice->>MultisigPallet: submit_transaction(multisig, add_owner(multisig, Dave))
    Bob->>MultisigPallet: confirm_transaction(multisig, transaction_id)
    MultisigPallet->>Runtime: dispatch(add_owner(multisig, Dave))
    MultisigPallet-->>Bob: OwnerAdded event
```
## Flow Diagrams for each extrinsic

## Create Multisig Flow

```mermaid
flowchart TD
    A([Start: create_multisig]) --> B{Validate Parameters}
    B -->|Invalid| C[Error: InvalidThreshold, TooManyOwners]
    B -->|Valid| D{Check for Duplicate Owners}
    D -->|Duplicates Found| E[Error: AlreadyOwner]
    D -->|No Duplicates| F[Generate Unique Multisig Account ID]
    F --> G[Calculate Required Deposit]
    G --> H{Hold Deposit from Creator}
    H -->|Failed| I[Error: FailedToReserveDeposit]
    H -->|Success| J[Create Multisig Data Structure]
    J --> K[Store Multisig in Storage]
    K --> L[Map Each Owner to Multisig]
    L --> M[Emit MultisigCreated Event]
    M --> N([End: Multisig Created])

    style A fill:#f9f,stroke:#333,stroke-width:2px
    style N fill:#bfb,stroke:#333,stroke-width:2px
    style C fill:#fbb,stroke:#333,stroke-width:2px
    style E fill:#fbb,stroke:#333,stroke-width:2px
    style I fill:#fbb,stroke:#333,stroke-width:2px
```

## Submit Transaction Flow

```mermaid
flowchart TD
    A([Start: submit_transaction]) --> B{Verify Multisig Exists}
    B -->|Not Found| C[Error: MultisigNotFound]
    B -->|Found| D{Verify Caller is Owner}
    D -->|Not Owner| E[Error: NotOwner]
    D -->|Is Owner| F{Verify Call is Allowed}
    F -->|Not Allowed| G[Error: CallNotAllowed]
    F -->|Allowed| H[Get Transaction ID from Counter]
    H --> I[Increment Transaction Counter]
    I --> J[Create & Store Transaction]
    J --> K[Auto-Confirm for Submitter]
    K --> L[Emit TransactionSubmitted Event]
    L --> M[Emit TransactionSigned Event]
    M --> N{Check if Threshold Met}
    N -->|Not Met| O[Wait for More Confirmations]
    N -->|Met| P[Execute Transaction]
    P --> Q[Emit TransactionExecuted Event]
    Q --> R([End: Transaction Processed])
    O --> R
    
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style R fill:#bfb,stroke:#333,stroke-width:2px
    style C fill:#fbb,stroke:#333,stroke-width:2px
    style E fill:#fbb,stroke:#333,stroke-width:2px
    style G fill:#fbb,stroke:#333,stroke-width:2px
```


## Confirm Transaction Flow

```mermaid
flowchart TD
    A([Start: confirm_transaction]) --> B{Verify Multisig Exists}
    B -->|Not Found| C[Error: MultisigNotFound]
    B -->|Found| D{Verify Caller is Owner}
    D -->|Not Owner| E[Error: NotOwner]
    D -->|Is Owner| F{Verify Transaction Exists}
    F -->|Not Found| G[Error: TransactionNotFound]
    F -->|Found| H{Transaction Already Executed?}
    H -->|Yes| I[Error: AlreadyExecuted]
    H -->|No| J{Already Confirmed by Caller?}
    J -->|Yes| K[Error: AlreadyConfirmed]
    J -->|No| L[Store Confirmation]
    L --> M[Emit TransactionSigned Event]
    M --> N{Check if Threshold Met}
    N -->|Not Met| O[Wait for More Confirmations]
    N -->|Met| P[Execute Transaction]
    P --> Q[Emit TransactionExecuted Event]
    Q --> R([End: Confirmation Processed])
    O --> R
    
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style R fill:#bfb,stroke:#333,stroke-width:2px
    style C fill:#fbb,stroke:#333,stroke-width:2px
    style E fill:#fbb,stroke:#333,stroke-width:2px
    style G fill:#fbb,stroke:#333,stroke-width:2px
    style I fill:#fbb,stroke:#333,stroke-width:2px
    style K fill:#fbb,stroke:#333,stroke-width:2px

```

## Revoke Confirmation Flow

```mermaid
flowchart TD
    A([Start: revoke_confirmation]) --> B{Verify Multisig Exists}
    B -->|Not Found| C[Error: MultisigNotFound]
    B -->|Found| D{Verify Caller is Owner}
    D -->|Not Owner| E[Error: NotOwner]
    D -->|Is Owner| F{Verify Transaction Exists}
    F -->|Not Found| G[Error: TransactionNotFound]
    F -->|Found| H{Transaction Already Executed?}
    H -->|Yes| I[Error: AlreadyExecuted]
    H -->|No| J{Has Caller Confirmed?}
    J -->|No| K[Error: NotConfirmed]
    J -->|Yes| L[Remove Confirmation]
    L --> M[Emit TransactionRevoked Event]
    M --> N([End: Confirmation Revoked])
    
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style N fill:#bfb,stroke:#333,stroke-width:2px
    style C fill:#fbb,stroke:#333,stroke-width:2px
    style E fill:#fbb,stroke:#333,stroke-width:2px
    style G fill:#fbb,stroke:#333,stroke-width:2px
    style I fill:#fbb,stroke:#333,stroke-width:2px
    style K fill:#fbb,stroke:#333,stroke-width:2px
```


## Add Owner Flow

```mermaid
flowchart TD
    A([Start: add_owner]) --> B{Verify Origin is Multisig}
    B -->|Not Multisig| C[Error: NotOwner]
    B -->|Is Multisig| D{Get Multisig Data}
    D -->|Not Found| E[Error: MultisigNotFound]
    D -->|Found| F{New Owner Already Owner?}
    F -->|Yes| G[Error: AlreadyOwner]
    F -->|No| H{Max Owners Exceeded?}
    H -->|Yes| I[Error: TooManyOwners]
    H -->|No| J[Add New Owner to List]
    J --> K[Update Multisig Storage]
    K --> L[Add Owner to Mapping]
    L --> M[Emit OwnerAdded Event]
    M --> N([End: Owner Added])

    style A fill:#f9f,stroke:#333,stroke-width:2px
    style N fill:#bfb,stroke:#333,stroke-width:2px
    style C fill:#fbb,stroke:#333,stroke-width:2px
    style E fill:#fbb,stroke:#333,stroke-width:2px
    style G fill:#fbb,stroke:#333,stroke-width:2px
    style I fill:#fbb,stroke:#333,stroke-width:2px
```

## Remove Owner Flow

```mermaid
flowchart TD
    A([Start: remove_owner]) --> B{Verify Origin is Multisig}
    B -->|Not Multisig| C[Error: NotOwner]
    B -->|Is Multisig| D{Get Multisig Data}
    D -->|Not Found| E[Error: MultisigNotFound]
    D -->|Found| F{Owner to Remove Exists?}
    F -->|No| G[Error: OwnerNotFound]
    F -->|Yes| H[Remove Owner from List]
    H --> I{New Threshold > Owner Count?}
    I -->|Yes| J[Adjust Threshold Downward]
    I -->|No| K[Keep Current Threshold]
    J --> L[Emit RequirementChanged Event]
    K --> M[Update Multisig Storage]
    L --> M
    M --> N[Remove Owner from Mapping]
    N --> O[Emit OwnerRemoved Event]
    O --> P([End: Owner Removed])

    style A fill:#f9f,stroke:#333,stroke-width:2px
    style P fill:#bfb,stroke:#333,stroke-width:2px
    style C fill:#fbb,stroke:#333,stroke-width:2px
    style E fill:#fbb,stroke:#333,stroke-width:2px
    style G fill:#fbb,stroke:#333,stroke-width:2px
```

## Replace Owner Flow

```mermaid
flowchart TD
    A([Start: replace_owner]) --> B{Verify Origin is Multisig}
    B -->|Not Multisig| C[Error: NotOwner]
    B -->|Is Multisig| D{Get Multisig Data}
    D -->|Not Found| E[Error: MultisigNotFound]
    D -->|Found| F{Old Owner Exists?}
    F -->|No| G[Error: OwnerNotFound]
    F -->|Yes| H{New Owner Already Owner?}
    H -->|Yes| I[Error: AlreadyOwner]
    H -->|No| J[Replace Old Owner with New Owner]
    J --> K[Update Multisig Storage]
    K --> L[Remove Old Owner from Mapping]
    L --> M[Add New Owner to Mapping]
    M --> N[Emit OwnerRemoved Event]
    N --> O[Emit OwnerAdded Event]
    O --> P([End: Owner Replaced])

    style A fill:#f9f,stroke:#333,stroke-width:2px
    style P fill:#bfb,stroke:#333,stroke-width:2px
    style C fill:#fbb,stroke:#333,stroke-width:2px
    style E fill:#fbb,stroke:#333,stroke-width:2px
    style G fill:#fbb,stroke:#333,stroke-width:2px
    style I fill:#fbb,stroke:#333,stroke-width:2px
```

## Destroy Multisig Flow

```mermaid
flowchart TD
    A([Start: destroy_multisig]) --> B{Get Multisig Data}
    B -->|Not Found| C[Error: MultisigNotFound]
    B -->|Found| D{Verify Caller is Owner}
    D -->|Not Owner| E[Error: NotAllowedToDissolve]
    D -->|Is Owner| F{Release Deposit to Creator}
    F -->|Failed| G[Error: FailedToReleaseDeposit]
    F -->|Success| H[Remove Multisig from Storage]
    H --> I[Remove All Owner Mappings]
    I --> J[Clean Up All Transactions]
    J --> K[Clean Up All Confirmations]
    K --> L[Emit MultisigDestroyed Event]
    L --> M([End: Multisig Destroyed])

    style A fill:#f9f,stroke:#333,stroke-width:2px
    style M fill:#bfb,stroke:#333,stroke-width:2px
    style C fill:#fbb,stroke:#333,stroke-width:2px
    style E fill:#fbb,stroke:#333,stroke-width:2px
    style G fill:#fbb,stroke:#333,stroke-width:2px
```

## Change Requirement Flow

```mermaid
flowchart TD
    A([Start: change_requirement]) --> B{Verify Origin is Multisig}
    B -->|Not Multisig| C[Error: NotOwner]
    B -->|Is Multisig| D{Get Multisig Data}
    D -->|Not Found| E[Error: MultisigNotFound]
    D -->|Found| F{Valid Threshold?}
    F -->|No| G[Error: InvalidThreshold]
    F -->|Yes| H[Update Threshold]
    H --> I[Update Multisig Storage]
    I --> J[Emit RequirementChanged Event]
    J --> K([End: Threshold Changed])

    style A fill:#f9f,stroke:#333,stroke-width:2px
    style K fill:#bfb,stroke:#333,stroke-width:2px
    style C fill:#fbb,stroke:#333,stroke-width:2px
    style E fill:#fbb,stroke:#333,stroke-width:2px
    style G fill:#fbb,stroke:#333,stroke-width:2px
```
