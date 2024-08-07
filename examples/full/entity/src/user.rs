//! `SeaORM` Entity, @generated by sea-orm-codegen 1.0.0-rc.7

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    #[sea_orm(unique)]
    pub username: String,
    #[sea_orm(unique)]
    pub email: String,
    pub password: String,
    pub email_confirmation_sent_at: Option<DateTimeWithTimeZone>,
    pub email_confirmation_token: Option<String>,
    pub email_confirmed_at: Option<DateTimeWithTimeZone>,
    pub last_sign_in_at: Option<DateTimeWithTimeZone>,
    pub recovery_sent_at: Option<DateTimeWithTimeZone>,
    pub recovery_token: Option<String>,
    pub email_change_sent_at: Option<DateTimeWithTimeZone>,
    pub email_change_token_new: Option<String>,
    pub email_change_token_current: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
