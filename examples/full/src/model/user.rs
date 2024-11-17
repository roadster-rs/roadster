use crate::model::entity::prelude::User;
use crate::model::entity::user;
use roadster::app::context::ProvideRef;
use roadster::error::RoadsterResult;
use sea_orm::{DatabaseConnection, EntityTrait};
use uuid::Uuid;

impl user::Model {
    pub async fn find_by_id(
        db: impl ProvideRef<DatabaseConnection>,
        id: Uuid,
    ) -> RoadsterResult<Self> {
        let user =
            User::find_by_id(id)
                .one(db.provide())
                .await?
                .ok_or(sea_orm::DbErr::RecordNotFound(format!(
                    "User with id {id} not found"
                )))?;

        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use crate::model::entity::user;
    use chrono::Utc;
    use roadster::app::context::MockProvideRef;
    use sea_orm::{DatabaseBackend, DatabaseConnection, MockDatabase};
    use uuid::Uuid;

    #[tokio::test]
    async fn find_by_id() {
        let id = Uuid::now_v7();
        let mut provide_db = MockProvideRef::<DatabaseConnection>::new();
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([vec![test_user(id)]])
            .into_connection();
        provide_db.expect_provide().return_const(db);

        let user = user::Model::find_by_id(provide_db, id).await.unwrap();

        assert_eq!(id, user.id);
        assert_eq!(id.to_string(), user.name);
        assert_eq!(id.to_string(), user.username);
    }

    fn test_user(id: Uuid) -> user::Model {
        let now = Utc::now();
        user::Model {
            id,
            created_at: now.into(),
            updated_at: now.into(),
            name: id.to_string(),
            username: id.to_string(),
            email: format!("{id}@example.com"),
            password: "password".to_string(),
            last_sign_in_at: now.into(),
            password_updated_at: now.into(),
            email_confirmation_sent_at: None,
            email_confirmation_token: None,
            email_confirmed_at: None,
            recovery_sent_at: None,
            recovery_token: None,
            email_change_sent_at: None,
            email_change_token_new: None,
            email_change_token_current: None,
            deleted_at: None,
        }
    }
}
