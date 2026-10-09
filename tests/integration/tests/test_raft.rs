use cluaizdb_consensus_raft::{RaftNode, RequestVoteArgs};

#[test]
fn test_raft_election_integration() {
    let node = RaftNode::new("node_1".to_string());
    
    let args = RequestVoteArgs {
        term: 1,
        candidate_id: "node_2".to_string(),
        last_log_index: 0,
        last_log_term: 0,
    };

    let reply = node.handle_request_vote(args);
    assert!(reply.vote_granted);
}
