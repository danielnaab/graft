# Raft Consensus

Status: stable

Raft is a consensus algorithm designed for understandability.

## Key concepts

- **Leader election**: One node is elected leader, handles all client requests
- **Log replication**: Leader replicates log entries to followers
- **Safety**: Only nodes with up-to-date logs can become leader

## How it works

1. Nodes start as followers
2. If a follower receives no heartbeat, it becomes a candidate
3. Candidate requests votes from other nodes
4. Node with majority votes becomes leader
5. Leader sends heartbeats to maintain authority

## See also

- [Paxos basics](paxos-basics.md) - an older consensus algorithm

## Sources

- [Raft paper (2014)](https://raft.github.io/raft.pdf) - original paper by Ongaro and Ousterhout
- [Raft visualization](https://raft.github.io/) - interactive explanation
