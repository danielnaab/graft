# Paxos Basics

Status: working

Paxos is a family of protocols for solving consensus in a network of unreliable processors.

## Key roles

- **Proposer**: Proposes values
- **Acceptor**: Votes on proposals
- **Learner**: Learns the chosen value

## Basic protocol

1. Proposer sends prepare request with proposal number
2. Acceptors promise not to accept lower-numbered proposals
3. Proposer sends accept request with value
4. Acceptors accept if they haven't promised to a higher number

## Challenges

- Complex to understand and implement correctly
- Multiple rounds may be needed
- Livelock possible without additional mechanisms

## See also

- [Raft consensus](raft-consensus.md) - designed to be more understandable

## Sources

- [Paxos Made Simple (2001)](https://lamport.azurewebsites.net/pubs/paxos-simple.pdf) - Lamport's simplified explanation
