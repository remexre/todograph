table! {
    deps (id) {
        id -> Nullable<Integer>,
        id_from -> Integer,
        id_to -> Integer,
    }
}

table! {
    todos (id) {
        id -> Integer,
        name -> Text,
        done -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(deps, todos);
