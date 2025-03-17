use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use uuid::Uuid;

pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub username: String,
}

impl User {
    pub(crate) async fn find_by_id(_state: &AppContext, _id: Uuid) -> RoadsterResult<User> {
        todo!()
    }
}
