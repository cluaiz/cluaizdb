use serde::{Deserialize, Serialize};
use cluaizdb_types::UniversalNeuron;

/// Raft RPC Messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftMessage {
    /// RequestVote RPC: Invoked by candidates to gather votes.
    RequestVote(RequestVoteArgs),
    /// RequestVote Reply
    RequestVoteReply(RequestVoteReply),
    /// AppendEntries RPC: Invoked by leader to replicate log entries and as a heartbeat.
    AppendEntries(AppendEntriesArgs),
    /// AppendEntries Reply
    AppendEntriesReply(AppendEntriesReply),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteArgs {
    pub term: u64,
    pub candidate_id: String,
    pub last_log_index: usize,
    pub last_log_term: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteReply {
    pub term: u64,
    pub vote_granted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesArgs {
    pub term: u64,
    pub leader_id: String,
    pub prev_log_index: usize,
    pub prev_log_term: u64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesReply {
    pub term: u64,
    pub success: bool,
    /// Optimization: allow follower to tell leader where to back up quickly
    pub conflict_index: Option<usize>,
    pub conflict_term: Option<u64>,
}

/// A single entry in the replicated log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub term: u64,
    pub index: usize,
    pub command: RaftCommand,
}

/// The command that gets applied to the state machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftCommand {
    /// Insert a new neuron
    InsertNeuron(UniversalNeuron),
    /// Delete a neuron
    DeleteNeuron(cluaizdb_types::NeuronId),
    /// Execute a generic DNA hook or config change
    ConfigChange(Vec<String>),
}
