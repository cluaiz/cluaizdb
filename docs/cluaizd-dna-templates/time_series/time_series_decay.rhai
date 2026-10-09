// cluaizdb-dna-templates/time_series_decay.rhai
// Automatic data lifecycle tiering and downsampling template

let age_ns = neuron.age_ns;
let day_ns = 86400000000000;
let week_ns = 604800000000000;

// 1. If older than 7 days, compress the entire record to Cold (ZSTD)
if age_ns > week_ns {
    return #{
        "new_tier": "Cold",
        "action": "CompressZstd"
    };
}

// 2. If older than 1 day, delete raw payload (Warm / Shadow state)
if age_ns > day_ns {
    return #{
        "new_tier": "Warm",
        "clear_payload": true
    };
}

return #{};
