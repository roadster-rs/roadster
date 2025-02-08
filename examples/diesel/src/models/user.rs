use crate::models::Timestamp;
use diesel::{Queryable, Selectable};
use uuid::Uuid;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::user)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub id: Uuid,
    pub name: String,
    pub username: String,
    pub email: String,
    pub password: String,
}
