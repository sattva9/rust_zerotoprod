use serde::{Deserialize, Serialize};

use super::{SubscriberEmail, SubscriberName};

#[derive(Serialize, Deserialize, Clone)]
pub struct Subscriber {
    pub name: SubscriberName,
    pub email: SubscriberEmail,
}
