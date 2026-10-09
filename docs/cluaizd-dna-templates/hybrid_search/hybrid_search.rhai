// cluaizdb-dna-templates/hybrid_search.rhai
// Hybrid vector and keyword relevance search template

let vec = vector_data;
let query = query_vector;
let doc_text = payload.to_lower();
let search_term = config.search_term.to_lower();

let text_score = 0.0;
if doc_text.contains(search_term) {
    text_score = 0.5; // Flat score boost for keyword match
}

let vector_score = 0.0;
if query.len() > 0 {
    // Exposes the native AVX/SIMD cosine distance helper
    vector_score = cosine_similarity(vec, query); 
}

// Combined weighted score: 70% vector relevance + 30% keyword relevance
let final_score = (vector_score * 0.7) + (text_score * 0.3);

return #{
    "score": final_score
};
