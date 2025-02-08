// @generated automatically by Diesel CLI.

diesel::table! {
    user (id) {
        created_at -> Timestamptz,
        id -> Uuid,
        name -> Varchar,
        username -> Varchar,
        email -> Varchar,
        password -> Varchar,
    }
}
