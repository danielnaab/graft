# Byzantine Fault Tolerance

Status: draft

Notes on handling Byzantine (arbitrary/malicious) failures in distributed systems.

## Key idea

Byzantine faults are failures where nodes can behave arbitrarily, including maliciously. Standard consensus algorithms like Paxos and Raft assume crash failures only.

## Open questions

- How does PBFT (Practical Byzantine Fault Tolerance) work?
- What's the relationship to blockchain consensus?
- When is BFT actually needed vs. simpler approaches?

## Sources

- [PBFT paper (1999)](https://pmg.csail.mit.edu/papers/osdi99.pdf) - Castro and Liskov
