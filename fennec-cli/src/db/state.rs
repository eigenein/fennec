use std::fmt::Debug;

use serde::{Serialize, de::DeserializeOwned};

pub trait ApplicationState: Debug + Serialize + DeserializeOwned {
    const ID: &str;
}
