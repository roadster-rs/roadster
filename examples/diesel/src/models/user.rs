use crate::models::Timestamp;
use diesel::{Insertable, Queryable, Selectable};
use uuid::Uuid;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub created_at: Timestamp,
    pub id: Uuid,
    pub name: String,
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::user)]
pub struct NewUser<'a> {
    name: &'a str,
    username: &'a str,
    email: &'a str,
    password: &'a str,
}

impl<'a> NewUser<'a> {
    pub fn new(name: &'a str, username: &'a str, email: &'a str, password: &'a str) -> Self {
        Self {
            name,
            username,
            email,
            password,
        }
    }
}
