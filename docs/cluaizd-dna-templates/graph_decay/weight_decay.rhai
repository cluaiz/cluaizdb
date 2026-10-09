// cluaizdb-dna-templates/weight_decay.rhai
// Semantic graph edge weight decay and pruning template

// on_read: increment connection weights on query access
let read_hook_result = #{
    "increase_weight": 0.05 // Increments weights of outgoing edges by 5%
};

// on_traverse: filter paths during recursive walks
let traverse_hook_result = #{};
let weight = edge_weight;
let threshold = config.min_weight_threshold;

if weight >= threshold {
    return #{
        "follow": true
    };
} else {
    return #{
        "follow": false
    };
}
