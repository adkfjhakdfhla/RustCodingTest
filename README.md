# Transaction processor
Streams transaction from a CSV file. Keeps a store of clients and transactions in-memory, dumps to stdout after finishing processing.

## Design decisions
The architecture is designed around using generic traits, allowing implementations to be easily swapped out.

For example, for simplicity, the store is using simple in-memory hashmaps. For a large-scale concurrent deployment, one might wish to use a store that is backed by some database system, which will provide atomic transactions. The APIs are provided by the traits.

For simplicity, the current solution runs in one process, though the code was designed async and should lend itself well to parallelisation (subject to atomicity considerations, discussed below). The `Runner` architecture means that, given an appropriate concurrency-capable implementation of `Store`, it should be trivial to spawn multiple `Runner`s.

Similarly, the `Runner` can be modified to accept other streams, or other data formats.

## Concurrency considerations
It's worth noting that the majority of events are independent, and thus could be parallelised with ease. The main dependencies are:
 - deposits must ensure any previous chargeback event has been processed;
 - withdrawals must ensure all chronologically preceeding deposits have been processed; and
 - disputes rely on a client's entire transaction history.

In short, for the same client, event processing should be largely chronological, though among different clients processing is entirely independent. Other than that, most operations on the transaction list will be read-only (with the exception of disputes - which would likely constitute a tiny minority of events), thus allowing efficient concurrent reads.

For a large-scale system, one would likely use a relational database, which would provide inherent support for atomic transactions and transaction ordering. Thus, for this task, no extra effort was dedicated to making event processing atomic. Instead, the processing of events was structured so as to be as easy to verify for correctness and test as possible, clearly organising the preconditions and postconditions for every change.

Use of a database would also allow better scaling by avoiding the need to keep the entire list of past transactions in memory.

## Omissions and areas for improvement
Other than implementation of a concurrent store, there are a few areas in which the solution can be improved, but was not in consideration of time.

#### Using fixed-point arithmetic for monetary amounts.
Currently floating-point is used which is unnecessary and has the potential to introduce floating point errors. This was not done because a fixed decimal library was not readily found and it was not considered important enough to dedicate time to a custom implementation.

#### Full testing.
In consideration of time, only a portion of unit tests were implemented. A full suit of tests for the `Processors` would follow much the same pattern, and provide better assurance of correctness.

Integration tests could also be used to test the entire program flow.

#### More robust CLI handling.
Very little effort was dedicated to parsing the command-line, or providing a human-friendly interface. If this was intended to be used as a CLI tool, proper error handling with user-friendly messages would be implemented, as well as potentially a more friendly interface (e.g. toggleable stderr output, multiple files as input, etc).

#### Better type safety for `Event`
As a detailed technical note: currently `Event` contains an `EventType`-enum field, and also an `amount: Option<f64>` field. However, the presence or absense of the `amount` is determined by the event's type (deposits and withdraws have an amount, the three dispute-related events don't).

Currently this requires explicit checks and test cases in the processing of events. In hindsight, it is likely possible to roll the entire `Event` data into the enum defining the five different types, thus guaranteeing that the exact data necessary for each event type would be present. However, this would likely complicate deserialization from CSV, and thus in the interest of time was left as-is.

## Design choices on ambiguous elements
A few areas in the spec were ambiguous, and the decisions taken are justified here.
###  Disputes providing a mismatching client and transaction ID
The spec asserts that the transaction ID in a dispute must exist, but disputes additionally provide a client ID. The choice was made to additionally restrict disputes to only be valid for correct and matching client IDs, as all transactions affect one client alone, and it would make little sense for one client to be able to open a dispute for another's transaction.

### Disputes on withdrawals
It is not specified whether withdrawals can be disputed. However, upon a withdrawal the amount leaves the system, and reversing that would require increasing the amount of funds in the system. In a real system, the withdrawal would be towards another bank or similar entity, and this is where the dispute would be resolved, with the target bank refunding the money back into the system.

Additionally, the phrasing in the specification (e.g. "on a dispute, the available funds should decrease while the held funds should increase") indicates that disputes are only intended for deposits. Given this, withdrawals are not subject to dispute, either. However, the system is built to be extensible, and changing this is trivial.

### Dispute limitations: double jeopardy, time limits
In some systems, there may be conditions on when a transaction can be disputed, such as a time limit beyond which that right is forfeited, or the inability to dispute the same transaction twice. As no such restriction was included in the specification, none were implemented here, though either would be simple to add if desired.
