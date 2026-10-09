// cluaizdb-dna-templates/crud_schema.rhai
// Strict CRUD validation rules template

let payload_str = payload; // Raw text payload
let doc = json(payload_str);
let action = config.action; // Target action: "Insert", "Update", "Delete"
let user_role = config.user_role; // User permission role

// 1. Enforce RBAC security boundaries
if action == "Delete" && user_role != "admin" {
    return #{
        "action": "Abort",
        "error": "Access Denied: Only Admin users can delete records."
    };
}

// 2. Enforce schema validations
if doc.username == "" || doc.email == "" {
    return #{
        "action": "Abort",
        "error": "Schema violation: username and email are required fields."
    };
}

// 3. Enforce field data types
if type_of(doc.age) != "int" {
    return #{
        "action": "Abort",
        "error": "Schema violation: age must be an Integer."
    };
}

return #{
    "action": "Allow",
    "sync_write": "lite"
};
