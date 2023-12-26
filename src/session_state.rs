use axum::extract::FromRequestParts;
use tower_sessions::Session;
use uuid::Uuid;

#[derive(FromRequestParts)]
pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    pub fn renew(&self) {
        self.0.cycle_id();
    }

    pub fn insert_user_id(&self, user_id: Uuid) -> anyhow::Result<()> {
        self.0
            .insert(Self::USER_ID_KEY, user_id)
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn get_user_id(&self) -> anyhow::Result<Option<Uuid>> {
        self.0
            .get(Self::USER_ID_KEY)
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn log_out(self) {
        self.0.flush()
    }
}
